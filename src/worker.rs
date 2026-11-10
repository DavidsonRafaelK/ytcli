use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::layout::Size;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::Protocol;

use crate::browser::Account;
use crate::error::Result;
use crate::feed::Feed;
use crate::model::{Thread, VideoCard, VideoDetails};
use crate::player::PlayerEvent;
use crate::{thumbnail, ytdlp};

/// Everything the UI thread waits for: input, and results coming back from background work.
pub enum Msg {
    /// A key press or a resize; nothing else ever needs a redraw.
    Input(io::Result<Event>),
    Player(PlayerEvent),
    Feed {
        generation: u64,
        result: Result<Vec<VideoCard>>,
    },
    Video {
        id: Box<str>,
        result: Result<VideoDetails>,
    },
    Comments {
        id: Box<str>,
        result: Result<Vec<Thread>>,
    },
    Related {
        id: Box<str>,
        result: Result<Vec<VideoCard>>,
    },
    Thumbnail {
        id: Box<str>,
        size: Size,
        result: Result<Protocol>,
    },
    Verified {
        account: Account,
        result: Result<()>,
    },
}

struct ThumbnailJob {
    id: Box<str>,
    size: Size,
}

/// Enough to fill a screen of thumbnails quickly without hammering the CDN.
const THUMBNAIL_THREADS: usize = 4;

/// Runs slow and blocking work off the UI thread. yt-dlp calls are rare and get a thread
/// each; thumbnails are many and share a small pool.
pub struct Worker {
    results: Sender<Msg>,
    thumbnails: Sender<ThumbnailJob>,
}

impl Worker {
    pub fn spawn(picker: &Picker) -> (Self, Receiver<Msg>) {
        let (results, inbox) = mpsc::channel();
        let (thumbnails, jobs) = mpsc::channel();
        let jobs = Arc::new(Mutex::new(jobs));
        for _ in 0..THUMBNAIL_THREADS {
            let jobs = Arc::clone(&jobs);
            let results = results.clone();
            let picker = picker.clone();
            thread::spawn(move || thumbnail_loop(&jobs, &results, &picker));
        }
        let input = results.clone();
        thread::spawn(move || input_loop(&input));
        thread::spawn(thumbnail::prune_cache);
        (
            Self {
                results,
                thumbnails,
            },
            inbox,
        )
    }

    /// Where results go, for others that report back to the UI thread.
    pub fn sender(&self) -> Sender<Msg> {
        self.results.clone()
    }

    pub fn feed(
        &self,
        generation: u64,
        feed: Feed,
        first: usize,
        count: usize,
        account: Option<Account>,
    ) {
        self.background(move || Msg::Feed {
            generation,
            result: ytdlp::feed_page(&feed, first, count, account.as_ref()),
        });
    }

    pub fn video(&self, id: Box<str>, account: Option<Account>) {
        self.background(move || {
            let result = ytdlp::video(&id, account.as_ref());
            Msg::Video { id, result }
        });
    }

    pub fn comments(&self, id: Box<str>, account: Option<Account>) {
        self.background(move || {
            let result = ytdlp::comments(&id, account.as_ref());
            Msg::Comments { id, result }
        });
    }

    pub fn related(&self, id: Box<str>, title: Box<str>, account: Option<Account>) {
        self.background(move || {
            let result = ytdlp::related(&id, &title, account.as_ref());
            Msg::Related { id, result }
        });
    }

    pub fn verify(&self, account: Account) {
        self.background(move || Msg::Verified {
            result: ytdlp::verify(&account),
            account,
        });
    }

    pub fn thumbnail(&self, id: Box<str>, size: Size) {
        // Sending only fails once the pool is gone, i.e. while the app shuts down.
        let _ = self.thumbnails.send(ThumbnailJob { id, size });
    }

    fn background(&self, job: impl FnOnce() -> Msg + Send + 'static) {
        let results = self.results.clone();
        thread::spawn(move || {
            let _ = results.send(job());
        });
    }
}

fn thumbnail_loop(jobs: &Mutex<Receiver<ThumbnailJob>>, results: &Sender<Msg>, picker: &Picker) {
    loop {
        // The lock is held only while waiting for a job, never while working on one.
        let job = match jobs.lock() {
            Ok(jobs) => jobs.recv(),
            Err(_) => return,
        };
        let Ok(ThumbnailJob { id, size }) = job else {
            return;
        };
        let result = thumbnail::render(&id, size, picker);
        if results.send(Msg::Thumbnail { id, size, result }).is_err() {
            return;
        }
    }
}

fn input_loop(results: &Sender<Msg>) {
    loop {
        let event = event::read();
        let wanted = match &event {
            Ok(Event::Key(key)) => key.kind == KeyEventKind::Press,
            Ok(Event::Resize(..)) | Err(_) => true,
            Ok(_) => false,
        };
        let failed = event.is_err();
        if wanted && results.send(Msg::Input(event)).is_err() || failed {
            return;
        }
    }
}
