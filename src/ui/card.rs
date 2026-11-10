use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui_image::Image;
use ratatui_image::protocol::Protocol;

use super::{text, theme};
use crate::format;
use crate::model::VideoCard;

/// One video in the grid: thumbnail with its duration badge, then title, channel and
/// views, led by `label` such as "Queued".
pub fn render(
    frame: &mut Frame,
    area: Rect,
    card: &VideoCard,
    selected: bool,
    label: Option<&'static str>,
    thumbnail: Option<&Protocol>,
    thumbnail_height: u16,
) {
    if selected {
        let border = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme::accent());
        frame.render_widget(border, area);
    }
    let inner = area.inner(Margin::new(1, 1));
    let [picture, info] =
        Layout::vertical([Constraint::Length(thumbnail_height), Constraint::Fill(1)]).areas(inner);

    match thumbnail {
        Some(image) => frame.render_widget(Image::new(image).allow_clipping(true), picture),
        None => frame.render_widget(Block::new().style(Style::new().bg(theme::SURFACE)), picture),
    }
    if let Some(secs) = card.duration.filter(|secs| *secs > 0) {
        badge(frame, picture, &format::duration(secs));
    }

    let width = usize::from(info.width);
    let title_style = if selected {
        theme::bold().fg(Color::White)
    } else {
        theme::bold()
    };
    let mut lines: Vec<Line> = text::clamp(&card.title, width, 2)
        .into_iter()
        .map(|line| Line::styled(line, title_style))
        .collect();
    lines.push(Line::styled(
        text::ellipsize(&card.channel, width),
        theme::muted(),
    ));
    lines.extend(meta_line(card.views, label));
    frame.render_widget(Paragraph::new(lines), info);
}

/// The views line, led by a highlighted `label`.
pub fn meta_line(views: Option<u64>, label: Option<&'static str>) -> Option<Line<'static>> {
    let views = views.map(format::views);
    let Some(label) = label else {
        return views.map(|views| Line::styled(views, theme::muted()));
    };
    let mut spans = vec![Span::styled(label, theme::highlight())];
    if let Some(views) = views {
        spans.push(Span::styled(format!(" · {views}"), theme::muted()));
    }
    Some(Line::from(spans))
}

/// Text drawn over the picture's bottom-right corner, hiding the image cells under it.
pub fn badge(frame: &mut Frame, picture: Rect, label: &str) {
    let label = format!(" {label} ");
    let width = u16::try_from(text::width(&label))
        .unwrap_or(0)
        .min(picture.width);
    if picture.height < 2 || width == 0 {
        return;
    }
    let area = Rect {
        x: picture.right().saturating_sub(width + 1),
        y: picture.bottom().saturating_sub(2),
        width,
        height: 1,
    };
    let style = Style::new().fg(Color::White).bg(Color::Black);
    frame.render_widget(Line::styled(label, style), area);
}
