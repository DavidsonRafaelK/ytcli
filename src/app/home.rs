use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::Intent;
use crate::error::Result;
use crate::feed::Feed;
use crate::layout::Grid;
use crate::model::VideoCard;

pub const PAGE_SIZE: usize = 24;

/// The video grid of one feed.
pub struct Home {
    pub feed: Feed,
    pub cards: Vec<VideoCard>,
    pub selected: usize,
    /// First grid row on screen.
    pub top_row: usize,
    pub loading: bool,
    pub error: Option<Box<str>>,
    exhausted: bool,
    /// Bumped on every reset so pages of a feed we already left get dropped.
    generation: u64,
}

impl Home {
    pub fn new() -> Self {
        Self {
            feed: Feed::Home,
            cards: Vec::new(),
            selected: 0,
            top_row: 0,
            loading: false,
            error: None,
            exhausted: false,
            generation: 0,
        }
    }

    pub fn reset(&mut self, feed: Feed) {
        *self = Self {
            feed,
            generation: self.generation + 1,
            ..Self::new()
        };
    }

    /// Tells the pages of this feed from those of one we already left.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Claims the next page for loading, unless one is on its way or none is left.
    pub fn next_page(&mut self) -> bool {
        if self.loading || self.exhausted || self.error.is_some() {
            return false;
        }
        self.loading = true;
        true
    }

    pub fn receive(&mut self, generation: u64, page: Result<Vec<VideoCard>>) {
        if generation != self.generation {
            return;
        }
        self.loading = false;
        match page {
            Ok(cards) => {
                self.exhausted = cards.is_empty();
                self.cards.extend(cards);
            }
            Err(err) => self.error = Some(err.to_string().into()),
        }
    }

    pub fn fail(&mut self, message: &str) {
        self.loading = false;
        self.error = Some(message.into());
    }

    pub fn on_key(&mut self, key: KeyEvent, grid: &Grid) -> Option<Intent> {
        let columns = grid.columns;
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Right | KeyCode::Char('l') => self.select(self.selected + 1),
            KeyCode::Up | KeyCode::Char('k') if self.selected >= columns => {
                self.selected -= columns;
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_down(columns),
            KeyCode::PageDown => self.select(self.selected + grid.visible()),
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(grid.visible()),
            KeyCode::Home | KeyCode::Char('g') => self.selected = 0,
            KeyCode::End | KeyCode::Char('G') => self.select(usize::MAX),
            KeyCode::Enter => return self.cards.get(self.selected).cloned().map(Intent::Open),
            KeyCode::Char('a') => {
                return self
                    .cards
                    .get(self.selected)
                    .cloned()
                    .map(Intent::ToggleQueue);
            }
            KeyCode::Tab => return Some(Intent::SwitchFeed(self.feed.cycle(1))),
            KeyCode::BackTab => return Some(Intent::SwitchFeed(self.feed.cycle(-1))),
            KeyCode::Esc if matches!(self.feed, Feed::Search(_)) => {
                return Some(Intent::SwitchFeed(Feed::Home));
            }
            KeyCode::Char('/' | 's') => return Some(Intent::FocusSearch),
            KeyCode::Char('r') => return Some(Intent::Refresh),
            KeyCode::Char('L') => return Some(Intent::ShowLogin),
            KeyCode::Char('q') => return Some(Intent::Quit),
            _ => return None,
        }
        self.scroll_into_view(grid);
        self.near_end(columns).then_some(Intent::LoadMore)
    }

    pub fn scroll_into_view(&mut self, grid: &Grid) {
        let row = self.selected / grid.columns;
        if row < self.top_row {
            self.top_row = row;
        } else if row >= self.top_row + grid.rows {
            self.top_row = row + 1 - grid.rows;
        }
    }

    fn select(&mut self, index: usize) {
        self.selected = index.min(self.cards.len().saturating_sub(1));
    }

    /// Straight down, or to the last card when the row below is only partly filled.
    fn move_down(&mut self, columns: usize) {
        let last_row = self.cards.len().saturating_sub(1) / columns;
        if self.selected / columns < last_row {
            self.select(self.selected + columns);
        }
    }

    fn near_end(&self, columns: usize) -> bool {
        self.selected + columns * 2 >= self.cards.len()
    }
}
