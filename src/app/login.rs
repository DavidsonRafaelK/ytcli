use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::Intent;
use crate::browser::{self, Browser};

pub enum LoginState {
    Idle,
    Checking,
    Failed(Box<str>),
}

/// Picking the browser whose Google session yt borrows.
pub struct Login {
    pub browsers: Vec<&'static Browser>,
    pub selected: usize,
    pub state: LoginState,
}

impl Login {
    pub fn new(current: Option<&'static Browser>) -> Self {
        let browsers = browser::installed();
        let selected = current
            .and_then(|current| browsers.iter().position(|b| *b == current))
            .unwrap_or(0);
        Self {
            browsers,
            selected,
            state: LoginState::Idle,
        }
    }

    pub fn on_key(&mut self, key: KeyEvent, signed_in: bool) -> Option<Intent> {
        if matches!(self.state, LoginState::Checking) {
            return None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1).min(self.browsers.len().saturating_sub(1));
            }
            KeyCode::Char('o') => return self.browser().map(Intent::OpenSignIn),
            KeyCode::Enter => return self.browser().map(Intent::Verify),
            KeyCode::Char('x') if signed_in => return Some(Intent::SignOut),
            KeyCode::Esc => return Some(Intent::Back),
            KeyCode::Char('q') => return Some(Intent::Quit),
            _ => {}
        }
        None
    }

    fn browser(&self) -> Option<&'static Browser> {
        self.browsers.get(self.selected).copied()
    }
}
