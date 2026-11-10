use std::collections::VecDeque;
use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::layout::Size;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::Protocol;

use crate::browser::Account;
use crate::error::Result;
use crate::feed::Feed;
use crate::model::{Thread, VideoCard, VideoDetails};
use crate::player::PlayerEvent;
use crate::ytdlp::FeedReader;
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

/// Thumbnails waiting for a thread: those on screen go before those fetched ahead.
#[derive(Default)]
struct ThumbnailQueue {
    jobs: Mutex<Jobs>,
    added: Condvar,
}

#[derive(Default)]
struct Jobs {
    on_screen: VecDeque<ThumbnailJob>,
    ahead: VecDeque<ThumbnailJob>,
}

impl ThumbnailQueue {
    fn push(&self, job: ThumbnailJob, on_screen: bool) {
        // Poisoned only if a thumbnail thread panicked; then nothing reads the queue.
        let Ok(mut jobs) = self.jobs.lock() else {
            return;
        };
        if on_screen {
            jobs.on_screen.push_back(job);
        } else {
            jobs.ahead.push_back(job);
        }
        self.added.notify_one();
    }

    /// Waits for a job; `None` only if another thumbnail thread panicked.
    fn pop(&self) -> Option<ThumbnailJob> {
        let mut jobs = self.jobs.lock().ok()?;
        loop {
            if let Some(job) = jobs.on_screen.pop_front() {
                return Some(job);
            }
            if let Some(job) = jobs.ahead.pop_front() {
                return Some(job);
            }
            jobs = self.added.wait(jobs).ok()?;
        }
    }

    fn clear(&self) {
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.on_screen.clear();
            jobs.ahead.clear();
        }
    }
}

/// Enough to fill a screen of thumbnails quickly without hammering the CDN.
const THUMBNAIL_THREADS: usize = 4;

/// How long a feed's yt-dlp may wait for the next page to be asked for.
const FEED_IDLE: Duration = Duration::from_secs(60);

/// Runs slow and blocking work off the UI thread. yt-dlp calls are rare and get a thread
/// each; thumbnails are many and share a small pool.
pub struct Worker {
    results: Sender<Msg>,
    thumbnails: Arc<ThumbnailQueue>,
    /// Asks the thread reading the current feed for a page; dropping it ends that thread.
    feed: Option<Sender<()>>,
}

impl Worker {
    pub fn spawn(picker: &Picker) -> (Self, Receiver<Msg>) {
        let (results, inbox) = mpsc::channel();
        let thumbnails = Arc::new(ThumbnailQueue::default());
        for _ in 0..THUMBNAIL_THREADS {
            let jobs = Arc::clone(&thumbnails);
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
                feed: None,
            },
            inbox,
        )
    }

    /// Where results go, for others that report back to the UI thread.
    pub fn sender(&self) -> Sender<Msg> {
        self.results.clone()
    }

    /// Makes `feed` the one [`Self::feed_page`] reads, in pages of `page_size` videos.
    pub fn open_feed(
        &mut self,
        generation: u64,
        feed: Feed,
        page_size: usize,
        account: Option<Account>,
    ) {
        let (pages, requests) = mpsc::channel();
        self.feed = Some(pages);
        let results = self.results.clone();
        let reader = FeedReader::new(feed, account);
        thread::spawn(move || feed_loop(generation, reader, page_size, &requests, &results));
    }

    /// Asks for the next page of the open feed.
    pub fn feed_page(&self) {
        if let Some(feed) = &self.feed {
            // Fails only if the feed thread panicked.
            let _ = feed.send(());
        }
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

    /// `on_screen` thumbnails are rendered before those of cards further down.
    pub fn thumbnail(&self, id: Box<str>, size: Size, on_screen: bool) {
        self.thumbnails.push(ThumbnailJob { id, size }, on_screen);
    }

    /// Drops the thumbnails still waiting; nobody is going to look at them.
    pub fn forget_thumbnails(&self) {
        self.thumbnails.clear();
    }

    fn background(&self, job: impl FnOnce() -> Msg + Send + 'static) {
        let results = self.results.clone();
        thread::spawn(move || {
            let _ = results.send(job());
        });
    }
}

/// Reads `reader` a page per request, until a newer feed replaces this one.
fn feed_loop(
    generation: u64,
    mut reader: FeedReader,
    page_size: usize,
    requests: &Receiver<()>,
    results: &Sender<Msg>,
) {
    loop {
        let request = if reader.is_waiting() {
            requests.recv_timeout(FEED_IDLE)
        } else {
            requests.recv().map_err(|_| RecvTimeoutError::Disconnected)
        };
        match request {
            Ok(()) => {
                let result = reader.page(page_size);
                if results.send(Msg::Feed { generation, result }).is_err() {
                    return;
                }
            }
            // Nobody scrolled on for a while.
            Err(RecvTimeoutError::Timeout) => reader.pause(),
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn thumbnail_loop(jobs: &ThumbnailQueue, results: &Sender<Msg>, picker: &Picker) {
    while let Some(ThumbnailJob { id, size }) = jobs.pop() {
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
