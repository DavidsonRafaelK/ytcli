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

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Rect, Size};
use ratatui_image::FontSize;

use crate::config::Config;
use crate::feed::Feed;
use crate::layout::{Areas, Grid, UpNext, WatchColumns};
use crate::model::VideoCard;
use crate::player::Player;
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
            login: Login::new(config.browser),
            thumbnails: Thumbnails::default(),
            config,
            status: None,
            font,
            quit: false,
            back_to: Screen::Home,
            terminal,
            player: Player::default(),
            worker,
        };
        if app.config.browser.is_none() {
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
                    self.watch
                        .as_mut()
                        .and_then(|watch| watch.on_key(key, rows))
                }
                Screen::Login => {
                    let signed_in = self.config.browser.is_some();
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
            Msg::Verified { browser, result } => match result {
                Ok(()) => {
                    self.config.browser = Some(browser);
                    self.save_config();
                    self.login.state = LoginState::Idle;
                    self.screen = Screen::Home;
                    self.load_feed(Feed::Home);
                }
                Err(err) => self.login.state = LoginState::Failed(err.to_string().into()),
            },
        }
    }

    /// Queues thumbnails for the cards on screen: the grid at home, "Up next" when watching.
    pub fn request_thumbnails(&mut self) {
        let (cards, first, count, size) = match (self.screen, &self.watch) {
            (Screen::Home, _) => {
                let grid = self.grid();
                let first = self.home.top_row * grid.columns;
                (&self.home.cards, first, grid.drawn(), grid.thumbnail)
            }
            (Screen::Watch, Some(watch)) => {
                let (Some(cards), Some(up_next)) = (watch.related.ready(), self.up_next()) else {
                    return;
                };
                (cards, watch.related_top, up_next.drawn(), up_next.thumbnail)
            }
            _ => return,
        };
        let wanted = cards.iter().skip(first).take(count);
        for card in wanted.clone() {
            if self.thumbnails.claim(&card.id, size) {
                self.worker.thumbnail(card.id.clone(), size);
            }
        }
        self.thumbnails.trim(wanted.map(|card| &*card.id));
    }

    fn apply(&mut self, intent: Intent) {
        match intent {
            Intent::Open(card) => self.open(card),
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
                self.login = Login::new(self.config.browser);
                self.back_to = self.screen;
                self.screen = Screen::Login;
            }
            Intent::OpenSignIn(browser) => {
                if let Err(err) = browser.open_sign_in() {
                    self.status = Some(format!("cannot open {}: {err}", browser.name).into());
                }
            }
            Intent::Verify(browser) => {
                self.login.state = LoginState::Checking;
                self.worker.verify(browser);
            }
            Intent::SignOut => {
                self.config.browser = None;
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

    fn open(&mut self, card: VideoCard) {
        self.play(&card.id);
        self.worker.video(card.id.clone(), self.config.browser);
        self.worker.comments(card.id.clone(), self.config.browser);
        self.worker
            .related(card.id.clone(), card.title.clone(), self.config.browser);
        self.watch = Some(Watch::new(card));
        self.screen = Screen::Watch;
    }

    fn play(&mut self, id: &str) {
        if let Err(err) = self.player.play(id, self.config.browser) {
            self.status = Some(format!("cannot start mpv: {err}").into());
        }
    }

    fn load_feed(&mut self, feed: Feed) {
        self.thumbnails.clear();
        let signed_out = feed.needs_login() && self.config.browser.is_none();
        self.home.reset(feed);
        if signed_out {
            self.home
                .fail("Sign in to see this feed. Press L, or / to search.");
        } else {
            self.load_more();
        }
    }

    fn load_more(&mut self) {
        if let Some((generation, first)) = self.home.next_page() {
            self.worker.feed(
                generation,
                self.home.feed.clone(),
                first,
                home::PAGE_SIZE,
                self.config.browser,
            );
        }
    }

    fn save_config(&mut self) {
        if let Err(err) = self.config.save() {
            self.status = Some(format!("cannot save settings: {err}").into());
        }
    }
}
