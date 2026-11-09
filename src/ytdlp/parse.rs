use serde::Deserialize;

use crate::error::Result;
use crate::model::{Comment, Thread, VideoCard, VideoDetails, is_video_id};

#[derive(Deserialize)]
struct RawPlaylist {
    #[serde(default)]
    entries: Vec<Option<RawEntry>>,
}

#[derive(Deserialize)]
struct RawEntry {
    id: Option<Box<str>>,
    title: Option<Box<str>>,
    channel: Option<Box<str>>,
    uploader: Option<Box<str>>,
    view_count: Option<u64>,
    duration: Option<f64>,
}

#[derive(Deserialize)]
struct RawVideo {
    title: Option<Box<str>>,
    channel: Option<Box<str>>,
    uploader: Option<Box<str>>,
    channel_is_verified: Option<bool>,
    channel_follower_count: Option<u64>,
    like_count: Option<u64>,
    view_count: Option<u64>,
    timestamp: Option<i64>,
    description: Option<Box<str>>,
    comment_count: Option<u64>,
}

#[derive(Deserialize)]
struct RawComments {
    #[serde(default)]
    comments: Vec<RawComment>,
}

#[derive(Deserialize)]
struct RawComment {
    id: Box<str>,
    parent: Box<str>,
    text: Option<Box<str>>,
    author: Option<Box<str>>,
    like_count: Option<u64>,
    #[serde(rename = "_time_text")]
    time_text: Option<Box<str>>,
    author_is_uploader: Option<bool>,
    is_pinned: Option<bool>,
}

pub fn cards(json: &[u8]) -> Result<Vec<VideoCard>> {
    let playlist: RawPlaylist = serde_json::from_slice(json)?;
    Ok(playlist
        .entries
        .into_iter()
        .flatten()
        .filter_map(RawEntry::into_card)
        .collect())
}

pub fn video(json: &[u8]) -> Result<VideoDetails> {
    let raw: RawVideo = serde_json::from_slice(json)?;
    Ok(VideoDetails {
        title: raw.title.unwrap_or_default(),
        channel: raw.channel.or(raw.uploader).unwrap_or_default(),
        verified: raw.channel_is_verified.unwrap_or(false),
        subscribers: raw.channel_follower_count,
        likes: raw.like_count,
        views: raw.view_count,
        uploaded: raw.timestamp,
        description: raw.description.unwrap_or_default(),
        comment_count: raw.comment_count,
    })
}

pub fn comments(json: &[u8]) -> Result<Vec<Thread>> {
    let raw: RawComments = serde_json::from_slice(json)?;
    Ok(threads(raw.comments))
}

impl RawEntry {
    /// Feeds and searches also list channels, playlists and shelves; only videos are kept.
    fn into_card(self) -> Option<VideoCard> {
        let id = self.id.filter(|id| is_video_id(id))?;
        Some(VideoCard {
            id,
            title: self.title.unwrap_or_default(),
            channel: self.channel.or(self.uploader).unwrap_or_default(),
            views: self.view_count,
            duration: self.duration.map(|secs| secs.max(0.0) as u32),
        })
    }
}

/// yt-dlp lists comments flat, with every reply right after its thread's root.
fn threads(comments: Vec<RawComment>) -> Vec<Thread> {
    let mut threads: Vec<(Box<str>, Thread)> = Vec::new();
    for raw in comments {
        let is_root = &*raw.parent == "root";
        let parent = raw.parent.clone();
        let id = raw.id.clone();
        let comment = raw.into_comment();
        if is_root {
            threads.push((
                id,
                Thread {
                    root: comment,
                    replies: Vec::new(),
                },
            ));
        } else if let Some((_, thread)) = threads.iter_mut().rev().find(|(id, _)| *id == parent) {
            thread.replies.push(comment);
        }
    }
    threads.into_iter().map(|(_, thread)| thread).collect()
}

impl RawComment {
    fn into_comment(self) -> Comment {
        Comment {
            author: self.author.unwrap_or_default(),
            text: self.text.unwrap_or_default(),
            likes: self.like_count,
            age: self.time_text.unwrap_or_default(),
            by_uploader: self.author_is_uploader.unwrap_or(false),
            pinned: self.is_pinned.unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_videos() {
        let json = br#"{"entries":[
            {"id":"DhsrNWUwb80","title":"A","channel":"C","view_count":5,"duration":61.0},
            {"id":"UC-w4KNTbWQCY8ciFGeHn5XQ","title":"A channel"},
            null
        ]}"#;
        let cards = cards(json).unwrap_or_default();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards.first().map(|c| c.duration), Some(Some(61)));
    }

    #[test]
    fn groups_replies_under_their_thread() {
        let json = br#"{"title":"T","comments":[
            {"id":"a","parent":"root","text":"first","author_is_uploader":null},
            {"id":"a.1","parent":"a","text":"reply"},
            {"id":"b","parent":"root","text":"second"}
        ]}"#;
        let threads = comments(json).unwrap_or_default();
        let replies: Vec<_> = threads.iter().map(|t| t.replies.len()).collect();
        assert_eq!(replies, [1, 0]);
    }
}
