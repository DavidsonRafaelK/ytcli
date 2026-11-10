/// A list of videos the home screen can show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Feed {
    Home,
    Subscriptions,
    History,
    WatchLater,
    Liked,
    Search(Box<str>),
}

impl Feed {
    /// The chips under the search bar, in order.
    pub const TABS: [Self; 5] = [
        Self::Home,
        Self::Subscriptions,
        Self::History,
        Self::WatchLater,
        Self::Liked,
    ];

    pub fn label(&self) -> &str {
        match self {
            Self::Home => "Home",
            Self::Subscriptions => "Subscriptions",
            Self::History => "History",
            Self::WatchLater => "Watch later",
            Self::Liked => "Liked videos",
            Self::Search(query) => query,
        }
    }

    pub fn needs_login(&self) -> bool {
        !matches!(self, Self::Search(_))
    }

    /// The tab `step` positions away, wrapping around. Searches step from Home.
    pub fn cycle(&self, step: isize) -> Self {
        let len = Self::TABS.len();
        let current = Self::TABS.iter().position(|tab| tab == self).unwrap_or(0);
        let next = (current as isize + step).rem_euclid(len as isize) as usize;
        Self::TABS.get(next).cloned().unwrap_or(Self::Home)
    }

    /// What to hand yt-dlp. Searches are fixed-size playlists in yt-dlp, so they need to
    /// know how far the caller intends to read; `None` when that is open-ended.
    pub fn source(&self, last: Option<usize>) -> String {
        match self {
            Self::Home => ":ytrec".into(),
            Self::Subscriptions => ":ytsubs".into(),
            Self::History => ":ythis".into(),
            Self::WatchLater => ":ytwatchlater".into(),
            Self::Liked => "https://www.youtube.com/playlist?list=LL".into(),
            Self::Search(query) => match last {
                Some(last) => format!("ytsearch{last}:{query}"),
                None => format!("ytsearchall:{query}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_through_tabs() {
        assert_eq!(Feed::Home.cycle(1), Feed::Subscriptions);
        assert_eq!(Feed::Home.cycle(-1), Feed::Liked);
        assert_eq!(Feed::Liked.cycle(1), Feed::Home);
        assert_eq!(Feed::Search("x".into()).cycle(1), Feed::Subscriptions);
    }
}
