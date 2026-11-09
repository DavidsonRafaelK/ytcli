use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Intent;

/// The search box in the header.
#[derive(Default)]
pub struct SearchBar {
    pub text: String,
    pub focused: bool,
}

impl SearchBar {
    pub fn on_key(&mut self, key: KeyEvent) -> Option<Intent> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Enter => {
                let query = self.text.trim();
                if query.is_empty() {
                    return None;
                }
                self.focused = false;
                return Some(Intent::Search(query.into()));
            }
            KeyCode::Esc => self.focused = false,
            KeyCode::Backspace => {
                self.text.pop();
            }
            KeyCode::Char('u') if ctrl => self.text.clear(),
            KeyCode::Char('w') if ctrl => self.delete_word(),
            KeyCode::Char(c) if !ctrl => self.text.push(c),
            _ => {}
        }
        None
    }

    fn delete_word(&mut self) {
        let kept = self.text.trim_end().rfind(' ').map_or(0, |space| space + 1);
        self.text.truncate(kept);
    }
}
