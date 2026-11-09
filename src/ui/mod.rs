mod card;
mod chips;
mod footer;
mod grid;
mod header;
mod login;
mod search_bar;
mod text;
mod theme;
mod watch;

use ratatui::Frame;

use crate::app::{App, Screen};
use crate::layout::Areas;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let areas = Areas::new(frame.area(), app.screen == Screen::Home);
    header::render(frame, areas.header, app);
    match app.screen {
        Screen::Home => {
            chips::render(frame, areas.chips, &app.home.feed);
            grid::render(frame, areas.body, app);
        }
        Screen::Watch => {
            if let Some(watch) = app.watch.as_mut() {
                watch::render(frame, areas.body, watch, &app.thumbnails, app.font);
            }
        }
        Screen::Login => login::render(frame, areas.body, &app.login, app.config.browser),
    }
    footer::render(frame, areas.footer, app);
}
