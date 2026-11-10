use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::Intent;
use crate::browser::{self, Account};

pub enum LoginState {
    Idle,
    Checking,
    Failed(Box<str>),
}

/// Picking the browser profile whose Google session yt borrows.
pub struct Login {
    pub accounts: Vec<Account>,
    pub selected: usize,
    pub state: LoginState,
}

impl Login {
    pub fn new(current: Option<&Account>) -> Self {
        let accounts = browser::accounts();
        let selected = current
            .and_then(|current| accounts.iter().position(|account| account == current))
            .unwrap_or(0);
        Self {
            accounts,
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
                self.selected = (self.selected + 1).min(self.accounts.len().saturating_sub(1));
            }
            KeyCode::Char('o') => return self.account().map(Intent::OpenSignIn),
            KeyCode::Enter => return self.account().map(Intent::Verify),
            KeyCode::Char('x') if signed_in => return Some(Intent::SignOut),
            KeyCode::Esc => return Some(Intent::Back),
            KeyCode::Char('q') => return Some(Intent::Quit),
            _ => {}
        }
        None
    }

    fn account(&self) -> Option<Account> {
        self.accounts.get(self.selected).cloned()
    }
}
