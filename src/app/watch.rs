use std::ops::Range;

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::Intent;
use crate::error::Result;
use crate::model::{Thread, VideoCard, VideoDetails};

pub enum Loadable<T> {
    Loading,
    Ready(T),
    Failed(Box<str>),
}

impl<T> Loadable<T> {
    fn from_result(result: Result<T>) -> Self {
        match result {
            Ok(value) => Self::Ready(value),
            Err(err) => Self::Failed(err.to_string().into()),
        }
    }

    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            _ => None,
        }
    }
}

/// Lines moved per key press inside an item taller than the screen.
const SCROLL_STEP: usize = 3;

/// Which column of the watch page takes the arrow keys.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Page,
    UpNext,
}

/// The page under the video: info, description and comments, with "Up next" beside it:
/// the queue, then related videos. The focusable items of the page are the description (0)
/// and then each comment thread.
pub struct Watch {
    pub card: VideoCard,
    pub details: Loadable<VideoDetails>,
    pub comments: Loadable<Vec<Thread>>,
    pub related: Loadable<Vec<VideoCard>>,
    pub column: Column,
    /// Selected video of "Up next", and the first one on screen; both index the queue
    /// followed by the related videos.
    pub up_next_selected: usize,
    pub up_next_top: usize,
    pub focus: usize,
    pub description_open: bool,
    pub open_threads: Vec<bool>,
    pub scroll: usize,
    /// Written by the renderer: the lines each item covers, and the page height.
    pub item_lines: Vec<Range<usize>>,
    pub viewport: usize,
}

impl Watch {
    pub fn new(card: VideoCard) -> Self {
        Self {
            card,
            details: Loadable::Loading,
            comments: Loadable::Loading,
            related: Loadable::Loading,
            column: Column::Page,
            up_next_selected: 0,
            up_next_top: 0,
            focus: 0,
            description_open: false,
            open_threads: Vec::new(),
            scroll: 0,
            item_lines: Vec::new(),
            viewport: 0,
        }
    }

    pub fn receive_details(&mut self, details: Result<VideoDetails>) {
        self.details = Loadable::from_result(details);
    }

    pub fn receive_comments(&mut self, threads: Result<Vec<Thread>>) {
        if let Ok(threads) = &threads {
            self.open_threads = vec![false; threads.len()];
        }
        self.comments = Loadable::from_result(threads);
    }

    pub fn receive_related(&mut self, cards: Result<Vec<VideoCard>>) {
        self.related = Loadable::from_result(cards);
    }

    /// `up_next` is what the "Up next" column lists, and `up_next_rows` how many of them
    /// fit on screen, `None` when the terminal is too narrow to show the column.
    pub fn on_key(
        &mut self,
        key: KeyEvent,
        up_next_rows: Option<usize>,
        up_next: &[VideoCard],
    ) -> Option<Intent> {
        match key.code {
            KeyCode::Char('p') => return Some(Intent::Replay),
            KeyCode::Char('A') => return Some(Intent::ToggleAutoplay),
            KeyCode::Char('/' | 's') => return Some(Intent::FocusSearch),
            KeyCode::Char('L') => return Some(Intent::ShowLogin),
            KeyCode::Char('q') => return Some(Intent::Quit),
            KeyCode::Esc | KeyCode::Backspace => return Some(Intent::Back),
            _ => {}
        }
        match up_next_rows {
            Some(rows) if self.column == Column::UpNext => self.on_up_next_key(key, rows, up_next),
            Some(_) if matches!(key.code, KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab) => {
                self.column = Column::UpNext;
                None
            }
            _ => {
                self.column = Column::Page;
                self.on_page_key(key)
            }
        }
    }

    /// Keeps the selected "Up next" video on screen, e.g. after the terminal changed size.
    pub fn scroll_up_next(&mut self, rows: usize) {
        let selected = self.up_next_selected;
        self.up_next_top = if selected < self.up_next_top {
            selected
        } else if selected >= self.up_next_top + rows {
            selected + 1 - rows
        } else {
            self.up_next_top
        };
    }

    fn on_page_key(&mut self, key: KeyEvent) -> Option<Intent> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.down(),
            KeyCode::Up | KeyCode::Char('k') => self.up(),
            KeyCode::PageDown => (0..self.viewport / 2).for_each(|_| self.down()),
            KeyCode::PageUp => (0..self.viewport / 2).for_each(|_| self.up()),
            KeyCode::Home | KeyCode::Char('g') => (self.focus, self.scroll) = (0, 0),
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle(),
            KeyCode::Left => return Some(Intent::Back),
            _ => {}
        }
        None
    }

    fn on_up_next_key(
        &mut self,
        key: KeyEvent,
        rows: usize,
        cards: &[VideoCard],
    ) -> Option<Intent> {
        let last = cards.len().saturating_sub(1);
        let selected = cards.get(self.up_next_selected).cloned();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.up_next_selected += 1,
            KeyCode::Up | KeyCode::Char('k') => {
                self.up_next_selected = self.up_next_selected.saturating_sub(1);
            }
            KeyCode::PageDown => self.up_next_selected += rows,
            KeyCode::PageUp => self.up_next_selected = self.up_next_selected.saturating_sub(rows),
            KeyCode::Home | KeyCode::Char('g') => self.up_next_selected = 0,
            KeyCode::End | KeyCode::Char('G') => self.up_next_selected = last,
            KeyCode::Enter => return selected.map(Intent::Open),
            KeyCode::Char('a') => return selected.map(Intent::ToggleQueue),
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Tab => self.column = Column::Page,
            _ => {}
        }
        self.up_next_selected = self.up_next_selected.min(last);
        self.scroll_up_next(rows);
        None
    }

    /// Scrolls through a tall item before moving on to the next one.
    fn down(&mut self) {
        let Some(lines) = self.item_lines.get(self.focus) else {
            return;
        };
        if lines.end > self.scroll + self.viewport {
            self.scroll += SCROLL_STEP;
        } else if self.focus + 1 < self.item_lines.len() {
            self.focus += 1;
        }
    }

    fn up(&mut self) {
        let Some(lines) = self.item_lines.get(self.focus) else {
            return;
        };
        if lines.start < self.scroll {
            self.scroll = self.scroll.saturating_sub(SCROLL_STEP);
        } else {
            self.focus = self.focus.saturating_sub(1);
        }
    }

    fn toggle(&mut self) {
        match self.focus.checked_sub(1) {
            None => self.description_open = !self.description_open,
            Some(thread) => {
                if let Some(open) = self.open_threads.get_mut(thread) {
                    *open = !*open;
                }
            }
        }
    }
}
