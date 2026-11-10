use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use super::theme;
use crate::app::{Login, LoginState};
use crate::browser::Account;

const WIDTH: u16 = 72;

pub fn render(frame: &mut Frame, area: Rect, login: &Login, current: Option<&Account>) {
    let height = u16::try_from(login.accounts.len()).unwrap_or(0) + 16;
    let [column] = Layout::horizontal([Constraint::Max(WIDTH)])
        .flex(Flex::Center)
        .areas(area);
    let [panel] = Layout::vertical([Constraint::Max(height)])
        .flex(Flex::Center)
        .areas(column);

    let mut lines = vec![
        Line::styled("Sign in with your Google account", theme::bold()),
        Line::default(),
        Line::raw("yt uses the YouTube session of a browser profile:"),
        Line::raw(" 1. press o and sign in to Google in the selected profile"),
        Line::raw(" 2. come back and press ⏎ to use that profile's session"),
        Line::default(),
    ];
    lines.extend(login.accounts.iter().enumerate().map(|(i, account)| {
        let marker = if i == login.selected { "▸ " } else { "  " };
        let mut spans = vec![Span::raw(marker), Span::raw(account.label())];
        if let Some(email) = &account.email {
            spans.push(Span::styled(format!("  {email}"), theme::muted()));
        }
        if Some(account) == current {
            spans.push(Span::styled("  (signed in)", theme::muted()));
        }
        let line = Line::from(spans);
        if i == login.selected {
            line.bold()
        } else {
            line
        }
    }));
    lines.push(Line::default());
    lines.extend(state_lines(&login.state));
    lines.push(Line::default());
    lines.push(Line::styled(
        "Cookies are read from the browser on each request; yt never stores them.",
        theme::muted(),
    ));
    if current.is_some() {
        lines.push(Line::styled("x  sign out", theme::muted()));
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::muted())
        .padding(ratatui::widgets::Padding::new(2, 2, 1, 1));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        panel,
    );
}

fn state_lines(state: &LoginState) -> Vec<Line<'static>> {
    match state {
        LoginState::Idle => vec![Line::styled(
            "Safari needs Full Disk Access for your terminal (System Settings → Privacy).",
            theme::muted(),
        )],
        LoginState::Checking => vec![
            Line::styled("Checking the session…", theme::bold()),
            Line::styled(
                "macOS may ask to allow Keychain access; choose “Always Allow”.",
                theme::muted(),
            ),
        ],
        LoginState::Failed(err) => vec![
            Line::styled("Not signed in:", theme::accent()),
            Line::styled(err.to_string(), theme::accent()),
        ],
    }
}
