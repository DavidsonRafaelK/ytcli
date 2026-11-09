use std::io;
use std::process::{Child, Command, Stdio};
use std::sync::LazyLock;

use crate::browser::Browser;
use crate::videotoolbox::{self, AV1, VP9};
use crate::ytdlp;

/// Hardware decoding keeps CPU and battery use low; the smaller stream buffers keep RAM
/// low (mpv buffers up to 150 MiB ahead and 50 MiB behind by default).
const MPV_ARGS: [&str; 5] = [
    "--force-window=immediate",
    "--no-terminal",
    "--hwdec=auto-safe",
    "--demuxer-max-bytes=50MiB",
    "--demuxer-max-back-bytes=20MiB",
];

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

/// The mpv window. One video plays at a time and keeps playing while browsing.
#[derive(Default)]
pub struct Player {
    child: Option<Child>,
}

impl Player {
    pub fn play(&mut self, id: &str, browser: Option<&Browser>) -> io::Result<()> {
        self.stop();
        let mut command = Command::new("mpv");
        command.args(MPV_ARGS).arg(format!(
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
            .arg(ytdlp::watch_url(id))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Fails only when mpv already exited, which is what we want anyway.
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}
