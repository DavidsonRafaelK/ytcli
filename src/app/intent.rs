use crate::browser::Browser;
use crate::feed::Feed;
use crate::model::VideoCard;

/// What a screen asks the app to do after a key press. Screens only change their own
/// state; anything with side effects (processes, network, other screens) goes through here.
pub enum Intent {
    Open(VideoCard),
    Replay,
    Back,
    FocusSearch,
    Search(Box<str>),
    SwitchFeed(Feed),
    LoadMore,
    Refresh,
    ShowLogin,
    OpenSignIn(&'static Browser),
    Verify(&'static Browser),
    SignOut,
    Quit,
}
