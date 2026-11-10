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
            watch::render(frame, areas.body, app);
        }
        Screen::Login => login::render(frame, areas.body, &app.login, app.config.account.as_ref()),
    }
    footer::render(frame, areas.footer, app);
}
