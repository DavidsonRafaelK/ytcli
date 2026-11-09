/// What a feed or search result shows about a video.
#[derive(Debug, Clone)]
pub struct VideoCard {
    pub id: Box<str>,
    pub title: Box<str>,
    pub channel: Box<str>,
    pub views: Option<u64>,
    /// Seconds.
    pub duration: Option<u32>,
}

/// Everything the watch page shows below the player.
#[derive(Debug)]
pub struct VideoDetails {
    pub title: Box<str>,
    pub channel: Box<str>,
    pub verified: bool,
    pub subscribers: Option<u64>,
    pub likes: Option<u64>,
    pub views: Option<u64>,
    /// Unix seconds.
    pub uploaded: Option<i64>,
    pub description: Box<str>,
    pub comment_count: Option<u64>,
}

#[derive(Debug)]
pub struct Thread {
    pub root: Comment,
    pub replies: Vec<Comment>,
}

#[derive(Debug)]
pub struct Comment {
    pub author: Box<str>,
    pub text: Box<str>,
    pub likes: Option<u64>,
    /// Already relative ("9 months ago"), as YouTube reports it.
    pub age: Box<str>,
    pub by_uploader: bool,
    pub pinned: bool,
}

const VIDEO_ID_LEN: usize = 11;

/// Video ids end up in file names and URLs, so anything else is rejected outright.
pub fn is_video_id(id: &str) -> bool {
    id.len() == VIDEO_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_ids() {
        assert!(is_video_id("DhsrNWUwb80"));
        assert!(is_video_id("a-b_c-d_e-f"));
        assert!(!is_video_id("UC-w4KNTbWQCY8ciFGeHn5XQ"));
        assert!(!is_video_id("../../etc/x"));
    }
}
