use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::theme;
use crate::feed::Feed;

/// The row of feed buttons; a search shows up as an extra chip in front.
pub fn render(frame: &mut Frame, area: Rect, current: &Feed) {
    let mut spans = Vec::new();
    if let Feed::Search(query) = current {
        spans.push(chip(&format!("Search: {query}"), true));
        spans.push(Span::raw(" "));
    }
    for tab in &Feed::TABS {
        spans.push(chip(tab.label(), tab == current));
        spans.push(Span::raw(" "));
    }
    frame.render_widget(Line::from(spans), area);
}

fn chip(label: &str, selected: bool) -> Span<'static> {
    let style = if selected {
        Style::new().fg(Color::Black).bg(Color::White).bold()
    } else {
        Style::new().fg(Color::White).bg(theme::SURFACE)
    };
    Span::styled(format!(" {label} "), style)
}
