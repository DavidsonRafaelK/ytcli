//! Everything YouTube-facing goes through yt-dlp, so its extractors stay the single place
//! that has to keep up with YouTube changes.

mod command;
mod parse;

use crate::browser::Browser;
use crate::error::Result;
use crate::feed::Feed;
use crate::model::{Thread, VideoCard, VideoDetails};

/// Top threads, all their replies up to a handful per thread.
const COMMENT_LIMITS: &str = "youtube:max_comments=80,20,all,10;comment_sort=top";

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
    browser: Option<&Browser>,
) -> Result<Vec<VideoCard>> {
    let last = first.saturating_add(count.saturating_sub(1));
    playlist(&feed.source(last), first, last, browser)
}

/// What youtube.com lists next to video `id`: its Mix, which is personal when signed in.
/// Videos without a Mix get a search for their `title` instead.
pub fn related(id: &str, title: &str, browser: Option<&Browser>) -> Result<Vec<VideoCard>> {
    let mix = format!("{}&list=RD{id}", watch_url(id));
    // The Mix starts with the video itself, hence one more.
    let mut cards = playlist(&mix, 1, RELATED_COUNT + 1, browser).unwrap_or_default();
    if cards.iter().all(|card| &*card.id == id) {
        cards = feed_page(&Feed::Search(title.into()), 1, RELATED_COUNT + 1, browser)?;
    }
    cards.retain(|card| &*card.id != id);
    cards.truncate(RELATED_COUNT);
    Ok(cards)
}

pub fn video(id: &str, browser: Option<&Browser>) -> Result<VideoDetails> {
    let url = watch_url(id);
    let json = command::run(&["-J", "--skip-download", "--", &url], browser)?;
    parse::video(&json)
}

/// Fetched apart from [`video`] because it is slower, and because yt-dlp then replaces
/// `comment_count` with the number of comments it fetched.
pub fn comments(id: &str, browser: Option<&Browser>) -> Result<Vec<Thread>> {
    let url = watch_url(id);
    let json = command::run(
        &[
            "-J",
            "--skip-download",
            "--write-comments",
            "--extractor-args",
            COMMENT_LIMITS,
            "--",
            &url,
        ],
        browser,
    )?;
    parse::comments(&json)
}

/// Entries `first` to `last` (1-based, inclusive) of a playlist-like `source`.
fn playlist(
    source: &str,
    first: usize,
    last: usize,
    browser: Option<&Browser>,
) -> Result<Vec<VideoCard>> {
    let range = format!("{first}:{last}");
    let json = command::run(
        &["--flat-playlist", "-J", "-I", &range, "--", source],
        browser,
    )?;
    parse::cards(&json)
}

/// Succeeds when `browser` holds a signed-in Google session.
pub fn verify(browser: &Browser) -> Result<()> {
    // History refuses to load when signed out, unlike Home which silently comes back empty.
    command::run(
        &["--flat-playlist", "-J", "-I", "1:1", "--", ":ythis"],
        Some(browser),
    )
    .map(drop)
}
