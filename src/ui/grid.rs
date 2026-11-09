use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

use super::{card, theme};
use crate::app::App;
use crate::layout::Grid;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let home = &app.home;
    if home.cards.is_empty() {
        let message = match (&home.error, home.loading) {
            (Some(err), _) => Line::styled(err.to_string(), theme::accent()),
            (None, true) => Line::styled("Loading…", theme::muted()),
            (None, false) => Line::styled("Nothing here yet.", theme::muted()),
        };
        centered_message(frame, area, message);
        return;
    }

    let grid = Grid::new(area, app.font);
    let first = home.top_row * grid.columns;
    let visible = home.cards.iter().enumerate().skip(first).take(grid.drawn());
    for (slot, (index, video)) in visible.enumerate() {
        card::render(
            frame,
            grid.card(slot),
            video,
            index == home.selected,
            app.thumbnails.get(&video.id),
            grid.thumbnail.height,
        );
    }
}

fn centered_message(frame: &mut Frame, area: Rect, message: Line) {
    let [row] = Layout::vertical([Constraint::Length(2)])
        .flex(ratatui::layout::Flex::Center)
        .areas(area);
    frame.render_widget(
        Paragraph::new(message).centered().wrap(Wrap { trim: true }),
        row,
    );
}
