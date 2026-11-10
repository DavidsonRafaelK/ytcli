use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use std::{env, fs, thread};

use serde::Deserialize;

use crate::browser::Browser;
use crate::videotoolbox::{self, AV1, VP9};
use crate::worker::Msg;
use crate::ytdlp;

/// Hardware decoding keeps CPU and battery use low; the smaller stream buffers keep RAM
/// low (mpv buffers up to 150 MiB ahead and 50 MiB behind by default). Idling keeps the
/// window open between videos, so the next one plays in the same place.
const MPV_ARGS: [&str; 6] = [
    "--force-window=immediate",
    "--idle=yes",
    "--no-terminal",
    "--hwdec=auto-safe",
    "--demuxer-max-bytes=50MiB",
    "--demuxer-max-back-bytes=20MiB",
];

/// mpv opens its IPC socket right after starting; this waits for it up to five seconds.
const CONNECT_ATTEMPTS: u32 = 50;
const CONNECT_RETRY: Duration = Duration::from_millis(100);

/// yt-dlp's format sort: at most 1080p, in the best codec VideoToolbox decodes in
/// hardware, so playback never falls back to the CPU. AV1 already ranks first, but only
/// M3 and later Macs decode it in hardware.
static FORMAT_SORT: LazyLock<&str> = LazyLock::new(|| {
    if videotoolbox::decodes_in_hardware(AV1) {
        "res:1080"
    } else if videotoolbox::decodes_in_hardware(VP9) {
        "res:1080,vcodec:vp9"
    } else {
        "res:1080,vcodec:h264"
    }
});

/// What mpv reports, tagged with the mpv process it came from.
pub enum PlayerEvent {
    /// Playlist entry `entry` stopped playing, for `reason` ("eof", "stop", "error", ...).
    Ended {
        instance: u64,
        entry: u64,
        reason: Box<str>,
        error: Option<Box<str>>,
    },
    /// mpv exited, usually because its window was closed.
    Closed { instance: u64 },
}

/// What a [`PlayerEvent`] means for the video playing now.
pub enum Outcome {
    Finished,
    Failed(Box<str>),
}

/// The mpv window. One video plays at a time and keeps playing while browsing; mpv stays
/// open between videos and is told what to play next over its IPC socket.
pub struct Player {
    events: Sender<Msg>,
    mpv: Option<Mpv>,
    /// mpv processes started so far, so events of one already replaced can be told apart.
    instances: u64,
}

struct Mpv {
    child: Child,
    instance: u64,
    /// The session mpv was started with; another one needs a new mpv.
    browser: Option<&'static Browser>,
    socket: PathBuf,
    /// Filled in by the listener once the socket is connected.
    ipc: Arc<Mutex<Option<UnixStream>>>,
    /// Files loaded so far. mpv numbers its playlist entries the same way, from 1, so the
    /// latest entry is the video playing now.
    loads: u64,
}

#[derive(Deserialize)]
struct RawEvent {
    event: Option<Box<str>>,
    reason: Option<Box<str>>,
    playlist_entry_id: Option<u64>,
    file_error: Option<Box<str>>,
}

impl Player {
    pub fn new(events: Sender<Msg>) -> Self {
        Self {
            events,
            mpv: None,
            instances: 0,
        }
    }

    pub fn play(&mut self, id: &str, browser: Option<&'static Browser>) -> io::Result<()> {
        let url = ytdlp::watch_url(id);
        if let Some(mpv) = self.mpv.as_mut().filter(|mpv| mpv.browser == browser) {
            // Fails while mpv is still starting or after it quit; a new one does the job.
            if mpv.load(&url).is_ok() {
                return Ok(());
            }
        }
        self.stop();
        self.instances += 1;
        self.mpv = Some(Mpv::spawn(&url, browser, self.instances, &self.events)?);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut mpv) = self.mpv.take() {
            // Fails only when mpv already exited, which is what we want anyway.
            let _ = mpv.child.kill();
            let _ = mpv.child.wait();
            let _ = fs::remove_file(&mpv.socket);
        }
    }

    /// Interprets an event; those about videos or mpv processes already replaced are dropped.
    pub fn on_event(&mut self, event: PlayerEvent) -> Option<Outcome> {
        let mpv = self.mpv.as_ref()?;
        match event {
            PlayerEvent::Ended {
                instance,
                entry,
                reason,
                error,
            } if instance == mpv.instance && entry == mpv.loads => match &*reason {
                "eof" => Some(Outcome::Finished),
                "error" => Some(Outcome::Failed(
                    error.unwrap_or_else(|| "playback failed".into()),
                )),
                _ => None,
            },
            PlayerEvent::Closed { instance } if instance == mpv.instance => {
                self.stop();
                None
            }
            _ => None,
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Mpv {
    fn spawn(
        url: &str,
        browser: Option<&'static Browser>,
        instance: u64,
        events: &Sender<Msg>,
    ) -> io::Result<Self> {
        // Unique per process and instance, so a leftover mpv is never mistaken for ours.
        let socket = env::temp_dir().join(format!("yt-mpv-{}-{instance}.sock", process::id()));
        let _ = fs::remove_file(&socket);
        let mut ipc_server = OsString::from("--input-ipc-server=");
        ipc_server.push(&socket);

        let mut command = Command::new("mpv");
        command.args(MPV_ARGS).arg(ipc_server).arg(format!(
            "--ytdl-raw-options-append=format-sort={}",
            *FORMAT_SORT
        ));
        if let Some(browser) = browser {
            command.arg(format!(
                "--ytdl-raw-options-append=cookies-from-browser={}",
                browser.id
            ));
        }
        let child = command
            .arg("--")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let ipc = Arc::new(Mutex::new(None));
        let (slot, events, path) = (Arc::clone(&ipc), events.clone(), socket.clone());
        thread::spawn(move || listen(&path, &slot, &events, instance));
        Ok(Self {
            child,
            instance,
            browser,
            socket,
            ipc,
            loads: 1,
        })
    }

    /// Replaces the video playing now with `url`.
    fn load(&mut self, url: &str) -> io::Result<()> {
        let mut ipc = self
            .ipc
            .lock()
            .map_err(|_| io::Error::other("mpv listener panicked"))?;
        let stream = ipc
            .as_mut()
            .ok_or_else(|| io::Error::other("mpv is still starting"))?;
        let command = serde_json::json!({ "command": ["loadfile", url, "replace"] });
        writeln!(stream, "{command}")?;
        self.loads += 1;
        Ok(())
    }
}

/// Connects to mpv's socket and forwards its playback events until mpv exits.
fn listen(socket: &Path, slot: &Mutex<Option<UnixStream>>, events: &Sender<Msg>, instance: u64) {
    if let Some(stream) = connect(socket) {
        if let (Ok(writer), Ok(mut slot)) = (stream.try_clone(), slot.lock()) {
            *slot = Some(writer);
        }
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else {
                break;
            };
            let Ok(raw) = serde_json::from_str::<RawEvent>(&line) else {
                continue;
            };
            if raw.event.as_deref() != Some("end-file") {
                continue;
            }
            let ended = PlayerEvent::Ended {
                instance,
                entry: raw.playlist_entry_id.unwrap_or(0),
                reason: raw.reason.unwrap_or_default(),
                error: raw.file_error,
            };
            if events.send(Msg::Player(ended)).is_err() {
                return;
            }
        }
    }
    let _ = fs::remove_file(socket);
    let _ = events.send(Msg::Player(PlayerEvent::Closed { instance }));
}

fn connect(socket: &Path) -> Option<UnixStream> {
    for _ in 0..CONNECT_ATTEMPTS {
        if let Ok(stream) = UnixStream::connect(socket) {
            return Some(stream);
        }
        thread::sleep(CONNECT_RETRY);
    }
    None
}
