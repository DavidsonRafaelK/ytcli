use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{search_bar, theme};
use crate::app::App;

const SEARCH_MAX_WIDTH: u16 = 80;

/// Logo, search bar and account, like the top of youtube.com.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [logo, middle, account] = Layout::horizontal([
        Constraint::Length(13),
        Constraint::Fill(1),
        Constraint::Length(22),
    ])
    .areas(area);
    let [search] = Layout::horizontal([Constraint::Max(SEARCH_MAX_WIDTH)])
        .flex(ratatui::layout::Flex::Center)
        .areas(middle);

    let logo_line = Line::from(vec![
        Span::styled(
            " ▶ ",
            Style::new().fg(Color::White).bg(theme::ACCENT).bold(),
        ),
        Span::styled(" YouTube", theme::bold()),
    ]);
    frame.render_widget(logo_line, middle_row(logo));

    search_bar::render(frame, search, &app.search);

    let account_line = match app.config.browser {
        Some(browser) => Line::styled(browser.name, theme::muted()),
        None => Line::styled("Sign in (L)", theme::accent().bold()),
    };
    frame.render_widget(
        Paragraph::new(account_line).alignment(Alignment::Right),
        middle_row(account),
    );
}

fn middle_row(area: Rect) -> Rect {
    Rect {
        y: area.y + area.height / 2,
        height: area.height.min(1),
        ..area
    }
}
