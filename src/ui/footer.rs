use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use super::theme;
use crate::app::{App, Column, Screen};

/// Key hints on the left, status on the right.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(Line::styled(hints(app), theme::muted()), area);

    let status = if let Some(status) = &app.status {
        Some(Line::styled(status.to_string(), theme::accent()))
    } else if app.screen == Screen::Home && app.home.loading && !app.home.cards.is_empty() {
        Some(Line::styled("Loading more…", theme::muted()))
    } else if let (Screen::Home, Some(err), false) =
        (app.screen, &app.home.error, app.home.cards.is_empty())
    {
        Some(Line::styled(err.to_string(), theme::accent()))
    } else {
        None
    };
    if let Some(status) = status {
        frame.render_widget(Paragraph::new(status).alignment(Alignment::Right), area);
    }
}

fn hints(app: &App) -> &'static str {
    if app.search.focused {
        return " ⏎ search  esc cancel  ^U clear";
    }
    match app.screen {
        Screen::Home => {
            " ←↑↓→ move  ⏎ play  a queue  tab feeds  / search  r refresh  L account  q quit"
        }
        Screen::Watch => match app.watch.as_ref().map(|watch| watch.column) {
            Some(Column::UpNext) => {
                " ↑↓ move  ⏎ play  a queue  ← details  A autoplay  p play again  esc back  q quit"
            }
            _ if app.up_next().is_some() => {
                " ↑↓ move  ⏎ expand  → up next  A autoplay  p play again  / search  esc back  q quit"
            }
            _ => " ↑↓ move  ⏎ expand  A autoplay  p play again  / search  esc back  q quit",
        },
        Screen::Login => " ↑↓ choose  o open sign-in page  ⏎ use this profile  esc skip",
    }
}
