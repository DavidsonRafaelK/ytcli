//! Everything YouTube-facing goes through yt-dlp, so its extractors stay the single place
//! that has to keep up with YouTube changes.

mod command;
mod parse;

use command::Lines;

use crate::browser::Account;
use crate::error::Result;
use crate::feed::Feed;
use crate::model::{Thread, VideoCard, VideoDetails};

/// Everything shown about a video is in its watch page, and `web` is the client whose
/// answer comes embedded in that page. Playable formats are mpv's business, so the other
/// clients, the player script with its challenges and the manifests are all left out:
/// one request instead of five, and no JavaScript to run.
macro_rules! page_only {
    () => {
        "youtube:player_client=web;player_skip=js;skip=hls,dash,translated_subs"
    };
}
const PAGE_ONLY: &str = page_only!();

/// Top threads, all their replies up to a handful per thread.
const PAGE_ONLY_WITH_COMMENTS: &str =
    concat!(page_only!(), ";max_comments=80,20,all,10;comment_sort=top");

/// About as many as youtube.com shows next to a video before loading more.
const RELATED_COUNT: usize = 20;

pub fn watch_url(id: &str) -> String {
    format!("https://www.youtube.com/watch?v={id}")
}

/// Reads `count` videos of `feed`, starting at the 1-based position `first`.
pub fn feed_page(
    feed: &Feed,
    first: usize,
    count: usize,
    account: Option<&Account>,
) -> Result<Vec<VideoCard>> {
    let last = first.saturating_add(count.saturating_sub(1));
    playlist(&feed.source(Some(last)), first, last, account)
}

/// A feed read page by page from one yt-dlp that keeps walking it, so a later page is
/// already waiting or one request away, instead of a new yt-dlp starting over from the
/// top. yt-dlp reads a little ahead and then waits for us.
pub struct FeedReader {
    feed: Feed,
    account: Option<Account>,
    lines: Option<Lines>,
    /// Whether the yt-dlp running now got anywhere; one that then fails is replaced.
    proven: bool,
    /// Entries read so far, videos or not: where another yt-dlp picks up.
    entries: usize,
    ended: bool,
    line: String,
}

impl FeedReader {
    /// Starts yt-dlp only once the first page is asked for.
    pub fn new(feed: Feed, account: Option<Account>) -> Self {
        Self {
            feed,
            account,
            lines: None,
            proven: false,
            entries: 0,
            ended: false,
            line: String::new(),
        }
    }

    /// The next `count` videos; fewer, then none, once the feed ends.
    pub fn page(&mut self, count: usize) -> Result<Vec<VideoCard>> {
        let mut cards = Vec::with_capacity(count);
        while cards.len() < count && !self.ended {
            let lines = match &mut self.lines {
                Some(lines) => lines,
                running => running.insert(open(&self.feed, self.entries, self.account.as_ref())?),
            };
            match lines.next(&mut self.line) {
                Ok(true) => {
                    self.proven = true;
                    self.entries += 1;
                    cards.extend(parse::card(&self.line));
                }
                Ok(false) => self.ended = true,
                // Most likely it waited too long between two pages.
                Err(_) if self.proven => self.pause(),
                Err(err) => {
                    self.pause();
                    return Err(err);
                }
            }
        }
        Ok(cards)
    }

    /// Whether a yt-dlp is waiting for the next page to be asked for.
    pub fn is_waiting(&self) -> bool {
        self.lines.is_some() && !self.ended
    }

    /// Stops yt-dlp, which holds on to some 90 MiB; the next page starts another one
    /// where this one left off.
    pub fn pause(&mut self) {
        self.lines = None;
        self.proven = false;
    }
}

/// Lists `feed` from the 0-based entry `skip` on, one JSON object per line.
fn open(feed: &Feed, skip: usize, account: Option<&Account>) -> Result<Lines> {
    let range = format!("{}:", skip + 1);
    let source = feed.source(None);
    Lines::spawn(
        &[
            "--flat-playlist",
            "--lazy-playlist",
            "-j",
            "-I",
            &range,
            "--",
            &source,
        ],
        account,
    )
}

/// What youtube.com lists next to video `id`: its Mix, which is personal when signed in.
/// Videos without a Mix get a search for their `title` instead.
pub fn related(id: &str, title: &str, account: Option<&Account>) -> Result<Vec<VideoCard>> {
    let mix = format!("{}&list=RD{id}", watch_url(id));
    // The Mix starts with the video itself, hence one more.
    let mut cards = playlist(&mix, 1, RELATED_COUNT + 1, account).unwrap_or_default();
    if cards.iter().all(|card| &*card.id == id) {
        cards = feed_page(&Feed::Search(title.into()), 1, RELATED_COUNT + 1, account)?;
    }
    cards.retain(|card| &*card.id != id);
    cards.truncate(RELATED_COUNT);
    Ok(cards)
}

pub fn video(id: &str, account: Option<&Account>) -> Result<VideoDetails> {
    let url = watch_url(id);
    let json = command::run(
        &[
            "-J",
            "--skip-download",
            "--ignore-no-formats-error",
            "--extractor-args",
            PAGE_ONLY,
            "--",
            &url,
        ],
        account,
    )?;
    parse::video(&json)
}

/// Fetched apart from [`video`] because it is slower, and because yt-dlp then replaces
/// `comment_count` with the number of comments it fetched.
pub fn comments(id: &str, account: Option<&Account>) -> Result<Vec<Thread>> {
    let url = watch_url(id);
    let json = command::run(
        &[
            "-J",
            "--skip-download",
            "--ignore-no-formats-error",
            "--write-comments",
            "--extractor-args",
            PAGE_ONLY_WITH_COMMENTS,
            "--",
            &url,
        ],
        account,
    )?;
    parse::comments(&json)
}

/// Entries `first` to `last` (1-based, inclusive) of a playlist-like `source`.
fn playlist(
    source: &str,
    first: usize,
    last: usize,
    account: Option<&Account>,
) -> Result<Vec<VideoCard>> {
    let range = format!("{first}:{last}");
    // A Mix that does not exist comes back as its video; that one stays cheap this way.
    let json = command::run(
        &[
            "--flat-playlist",
            "-J",
            "-I",
            &range,
            "--ignore-no-formats-error",
            "--extractor-args",
            PAGE_ONLY,
            "--",
            source,
        ],
        account,
    )?;
    parse::cards(&json)
}

/// Succeeds when `account` holds a signed-in Google session.
pub fn verify(account: &Account) -> Result<()> {
    // History refuses to load when signed out, unlike Home which silently comes back empty.
    command::run(
        &["--flat-playlist", "-J", "-I", "1:1", "--", ":ythis"],
        Some(account),
    )
    .map(drop)
}
