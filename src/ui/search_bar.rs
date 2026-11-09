use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};

use super::{text, theme};
use crate::app::SearchBar;

pub fn render(frame: &mut Frame, area: Rect, bar: &SearchBar) {
    let border = if bar.focused {
        theme::FOCUS
    } else {
        theme::SURFACE
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let room = usize::from(inner.width.saturating_sub(2));
    let typed = text::width(&bar.text);
    let content = if bar.text.is_empty() && !bar.focused {
        Line::styled(" Search", theme::muted())
    } else {
        Line::raw(format!(" {}", bar.text))
    };
    // Keep the end of a long query in view, where the cursor is.
    let hidden = u16::try_from(typed.saturating_sub(room)).unwrap_or(u16::MAX);
    frame.render_widget(Paragraph::new(content).scroll((0, hidden)), inner);

    if bar.focused {
        let shown = u16::try_from(typed.min(room)).unwrap_or(0);
        frame.set_cursor_position(Position::new(inner.x + 1 + shown, inner.y));
    }
}
