mod home;
mod intent;
mod login;
mod search;
mod thumbnails;
mod watch;

pub use home::Home;
pub use intent::Intent;
pub use login::{Login, LoginState};
pub use search::SearchBar;
pub use thumbnails::Thumbnails;
pub use watch::{Column, Loadable, Watch};

use std::collections::HashSet;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Rect, Size};
use ratatui_image::FontSize;

use crate::config::Config;
use crate::feed::Feed;
use crate::layout::{Areas, Grid, UpNext, WatchColumns};
use crate::model::VideoCard;
use crate::player::{Outcome, Player};
use crate::worker::{Msg, Worker};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Watch,
    Login,
}

pub struct App {
    pub screen: Screen,
    pub home: Home,
    pub watch: Option<Watch>,
    pub search: SearchBar,
    pub login: Login,
    pub thumbnails: Thumbnails,
    pub config: Config,
    pub status: Option<Box<str>>,
    pub font: FontSize,
    pub quit: bool,
    /// Videos to play next, in order, before autoplay picks a related one.
    pub queue: Vec<VideoCard>,
    /// Videos started this session; autoplay skips them so it cannot go back and forth.
    played: HashSet<Box<str>>,
    /// Where Esc on the login screen returns to.
    back_to: Screen,
    terminal: Size,
    player: Player,
    worker: Worker,
}

impl App {
    pub fn new(worker: Worker, font: FontSize, terminal: Size) -> Self {
        let config = Config::load();
        let mut app = Self {
            screen: Screen::Home,
            home: Home::new(),
            watch: None,
            search: SearchBar::default(),
            login: Login::new(config.account.as_ref()),
            thumbnails: Thumbnails::default(),
            config,
            status: None,
            font,
            quit: false,
            queue: Vec::new(),
            played: HashSet::new(),
            back_to: Screen::Home,
            terminal,
            player: Player::new(worker.sender()),
            worker,
        };
        if app.config.account.is_none() {
            app.screen = Screen::Login;
        }
        app.load_feed(Feed::Home);
        app
    }

    pub fn grid(&self) -> Grid {
        let screen = Rect::new(0, 0, self.terminal.width, self.terminal.height);
        Grid::new(Areas::new(screen, true).body, self.font)
    }

    /// The "Up next" column of the watch page, if the terminal is wide enough for it.
    pub fn up_next(&self) -> Option<UpNext> {
        let screen = Rect::new(0, 0, self.terminal.width, self.terminal.height);
        WatchColumns::new(Areas::new(screen, false).body, self.font).up_next
    }

    pub fn resize(&mut self, terminal: Size) {
        self.terminal = terminal;
        let grid = self.grid();
        self.home.scroll_into_view(&grid);
        let rows = self.up_next().map(|up_next| up_next.rows());
        if let (Some(watch), Some(rows)) = (self.watch.as_mut(), rows) {
            watch.scroll_up_next(rows);
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        self.status = None;
        let intent = if self.search.focused {
            self.search.on_key(key)
        } else {
            match self.screen {
                Screen::Home => {
                    let grid = self.grid();
                    self.home.on_key(key, &grid)
                }
                Screen::Watch => {
                    let rows = self.up_next().map(|up_next| up_next.rows());
                    let cards: Vec<VideoCard> = self.up_next_cards().into_iter().cloned().collect();
                    self.watch
                        .as_mut()
                        .and_then(|watch| watch.on_key(key, rows, &cards))
                }
                Screen::Login => {
                    let signed_in = self.config.account.is_some();
                    self.login.on_key(key, signed_in)
                }
            }
        };
        if let Some(intent) = intent {
            self.apply(intent);
        }
    }

    pub fn on_msg(&mut self, msg: Msg) {
        match msg {
            // The main loop handles input itself, since a broken terminal ends the app.
            Msg::Input(_) => {}
            Msg::Player(event) => match self.player.on_event(event) {
                Some(Outcome::Finished) => self.play_next(),
                Some(Outcome::Failed(err)) => self.status = Some(format!("mpv: {err}").into()),
                None => {}
            },
            Msg::Feed { generation, result } => self.home.receive(generation, result),
            Msg::Video { id, result } => {
                if let Some(watch) = self.watching(&id) {
                    watch.receive_details(result);
                }
            }
            Msg::Comments { id, result } => {
                if let Some(watch) = self.watching(&id) {
                    watch.receive_comments(result);
                }
            }
            Msg::Related { id, result } => {
                if let Some(watch) = self.watching(&id) {
                    watch.receive_related(result);
                }
            }
            Msg::Thumbnail { id, size, result } => self.thumbnails.receive(id, size, result),
            Msg::Verified { account, result } => match result {
                Ok(()) => {
                    // mpv was started with the old session's cookies.
                    self.player.stop();
                    self.config.account = Some(account);
                    self.save_config();
                    self.login.state = LoginState::Idle;
                    self.screen = Screen::Home;
                    self.load_feed(Feed::Home);
                }
                Err(err) => self.login.state = LoginState::Failed(err.to_string().into()),
            },
        }
    }

    /// What the "Up next" column lists: the queue, then related videos not already in it.
    pub fn up_next_cards(&self) -> Vec<&VideoCard> {
        let Some(watch) = &self.watch else {
            return Vec::new();
        };
        let related = watch.related.ready().map_or(&[][..], Vec::as_slice);
        let unqueued = related.iter().filter(|card| !self.is_queued(&card.id));
        self.queue.iter().chain(unqueued).collect()
    }

    pub fn is_queued(&self, id: &str) -> bool {
        self.queue.iter().any(|card| &*card.id == id)
    }

    /// The video that plays when this one ends: the queue first, then with autoplay the
    /// first related video not played yet.
    pub fn next_up(&self) -> Option<&VideoCard> {
        if let Some(card) = self.queue.first() {
            return Some(card);
        }
        if !self.config.autoplay {
            return None;
        }
        let related = self.watch.as_ref()?.related.ready()?;
        related.iter().find(|card| !self.played.contains(&card.id))
    }

    /// Queues thumbnails for the cards on screen: the grid at home, "Up next" when watching.
    /// They are followed by as many of the cards after them, so scrolling on finds its
    /// thumbnails already there.
    pub fn request_thumbnails(&mut self) {
        let (ids, on_screen, size): (Vec<Box<str>>, usize, Size) = match (self.screen, &self.watch)
        {
            (Screen::Home, _) => {
                let grid = self.grid();
                let first = self.home.top_row * grid.columns;
                let wanted = self.home.cards.iter().skip(first).take(grid.drawn() * 2);
                (
                    wanted.map(|card| card.id.clone()).collect(),
                    grid.drawn(),
                    grid.thumbnail,
                )
            }
            (Screen::Watch, Some(watch)) => {
                let Some(up_next) = self.up_next() else {
                    return;
                };
                let cards = self.up_next_cards().into_iter();
                let wanted = cards.skip(watch.up_next_top).take(up_next.drawn() * 2);
                (
                    wanted.map(|card| card.id.clone()).collect(),
                    up_next.drawn(),
                    up_next.thumbnail,
                )
            }
            _ => return,
        };
        for (slot, id) in ids.iter().enumerate() {
            if self.thumbnails.claim(id, size) {
                self.worker.thumbnail(id.clone(), size, slot < on_screen);
            }
        }
        self.thumbnails.trim(ids.iter().map(|id| &**id));
    }

    fn apply(&mut self, intent: Intent) {
        match intent {
            Intent::Open(card) => self.open(card),
            Intent::ToggleQueue(card) => self.toggle_queue(&card),
            Intent::ToggleAutoplay => {
                self.config.autoplay = !self.config.autoplay;
                self.save_config();
                let state = if self.config.autoplay { "on" } else { "off" };
                self.status = Some(format!("Autoplay {state}").into());
            }
            Intent::Replay => {
                if let Some(id) = self.watch.as_ref().map(|watch| watch.card.id.clone()) {
                    self.play(&id);
                }
            }
            Intent::Back => {
                self.screen = match self.screen {
                    Screen::Login => self.back_to,
                    _ => Screen::Home,
                };
                self.login.state = LoginState::Idle;
            }
            Intent::FocusSearch => self.search.focused = true,
            Intent::Search(query) => {
                self.screen = Screen::Home;
                self.load_feed(Feed::Search(query));
            }
            Intent::SwitchFeed(feed) => {
                self.search.text.clear();
                self.load_feed(feed);
            }
            Intent::LoadMore => self.load_more(),
            Intent::Refresh => self.load_feed(self.home.feed.clone()),
            Intent::ShowLogin => {
                self.login = Login::new(self.config.account.as_ref());
                self.back_to = self.screen;
                self.screen = Screen::Login;
            }
            Intent::OpenSignIn(account) => {
                if let Err(err) = account.open_sign_in() {
                    self.status = Some(format!("cannot open {}: {err}", account.label()).into());
                }
            }
            Intent::Verify(account) => {
                self.login.state = LoginState::Checking;
                self.worker.verify(account);
            }
            Intent::SignOut => {
                self.config.account = None;
                self.save_config();
                self.player.stop();
                self.login = Login::new(None);
                self.load_feed(Feed::Home);
            }
            Intent::Quit => self.quit = true,
        }
    }

    /// The watch page, if it still shows video `id`.
    fn watching(&mut self, id: &str) -> Option<&mut Watch> {
        self.watch.as_mut().filter(|watch| &*watch.card.id == id)
    }

    fn toggle_queue(&mut self, card: &VideoCard) {
        if self.is_queued(&card.id) {
            self.queue.retain(|queued| queued.id != card.id);
            self.status = Some("Removed from queue".into());
        } else {
            self.queue.push(card.clone());
            self.status = Some(format!("Added to queue ({})", self.queue.len()).into());
        }
        // The list just reordered; keep "Up next" on the same video.
        let position = self.up_next_cards().iter().position(|up| up.id == card.id);
        let rows = self.up_next().map(|up_next| up_next.rows());
        if let (Screen::Watch, Some(watch), Some(position), Some(rows)) =
            (self.screen, self.watch.as_mut(), position, rows)
        {
            watch.up_next_selected = position;
            watch.scroll_up_next(rows);
        }
    }

    /// Called when a video played to its end.
    fn play_next(&mut self) {
        let Some(next) = self.next_up().cloned() else {
            return;
        };
        let screen = self.screen;
        self.open(next);
        // Like youtube.com the watch page follows along, but browsing elsewhere goes on.
        if screen != Screen::Watch {
            self.screen = screen;
        }
    }

    fn open(&mut self, card: VideoCard) {
        self.queue.retain(|queued| queued.id != card.id);
        self.played.insert(card.id.clone());
        self.play(&card.id);
        self.worker
            .video(card.id.clone(), self.config.account.clone());
        self.worker
            .comments(card.id.clone(), self.config.account.clone());
        self.worker.related(
            card.id.clone(),
            card.title.clone(),
            self.config.account.clone(),
        );
        self.watch = Some(Watch::new(card));
        self.screen = Screen::Watch;
    }

    fn play(&mut self, id: &str) {
        if let Err(err) = self.player.play(id, self.config.account.as_ref()) {
            self.status = Some(format!("cannot start mpv: {err}").into());
        }
    }

    fn load_feed(&mut self, feed: Feed) {
        self.thumbnails.clear();
        self.worker.forget_thumbnails();
        let signed_out = feed.needs_login() && self.config.account.is_none();
        self.home.reset(feed);
        // Also ends the yt-dlp of the feed shown until now.
        self.worker.open_feed(
            self.home.generation(),
            self.home.feed.clone(),
            home::PAGE_SIZE,
            self.config.account.clone(),
        );
        if signed_out {
            self.home
                .fail("Sign in to see this feed. Press L, or / to search.");
        } else {
            self.load_more();
        }
    }

    fn load_more(&mut self) {
        if self.home.next_page() {
            self.worker.feed_page();
        }
    }

    fn save_config(&mut self) {
        if let Err(err) = self.config.save() {
            self.status = Some(format!("cannot save settings: {err}").into());
        }
    }
}
