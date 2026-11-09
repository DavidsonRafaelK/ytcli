use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui_image::Image;

use crate::app::{Column, Loadable, Thumbnails, Watch};
use crate::format;
use crate::layout::{Row, UpNext};
use crate::model::VideoCard;
use crate::ui::{card, text, theme};

/// Related videos beside the page, one row each: thumbnail, then title, channel and views.
pub fn render(frame: &mut Frame, up_next: &UpNext, watch: &Watch, thumbnails: &Thumbnails) {
    frame.render_widget(Line::styled("Up next", theme::bold()), up_next.heading);
    let message = match &watch.related {
        Loadable::Loading => Line::styled("Loading…", theme::muted()),
        Loadable::Failed(err) => Line::styled(err.to_string(), theme::accent()),
        Loadable::Ready(cards) if cards.is_empty() => {
            Line::styled("No related videos.", theme::muted())
        }
        Loadable::Ready(cards) => {
            let focused = watch.column == Column::UpNext;
            let visible = cards.iter().enumerate().skip(watch.related_top);
            for (slot, (index, video)) in visible.take(up_next.drawn()).enumerate() {
                let selected = focused && index == watch.related_selected;
                row(frame, &up_next.row(slot), video, selected, thumbnails);
            }
            return;
        }
    };
    // Lined up with the titles of the rows.
    let area = up_next.row(0).picture;
    let area = Rect {
        width: up_next.heading.right().saturating_sub(area.x),
        ..area
    };
    frame.render_widget(Paragraph::new(message).wrap(Wrap { trim: true }), area);
}

fn row(frame: &mut Frame, row: &Row, video: &VideoCard, selected: bool, thumbnails: &Thumbnails) {
    if selected {
        let bar = vec![Line::styled("▌", theme::accent()); usize::from(row.gutter.height)];
        frame.render_widget(Paragraph::new(bar), row.gutter);
    }

    match thumbnails.get(&video.id) {
        Some(image) => frame.render_widget(Image::new(image).allow_clipping(true), row.picture),
        None => frame.render_widget(
            Block::new().style(Style::new().bg(theme::SURFACE)),
            row.picture,
        ),
    }
    if let Some(secs) = video.duration.filter(|secs| *secs > 0) {
        card::badge(frame, row.picture, &format::duration(secs));
    }

    let width = usize::from(row.text.width);
    let title_style = if selected {
        theme::bold().fg(Color::White)
    } else {
        theme::bold()
    };
    let mut lines: Vec<Line> = text::clamp(&video.title, width, 2)
        .into_iter()
        .map(|line| Line::styled(line, title_style))
        .collect();
    lines.push(Line::styled(
        text::ellipsize(&video.channel, width),
        theme::muted(),
    ));
    if let Some(views) = video.views {
        lines.push(Line::styled(format::views(views), theme::muted()));
    }
    frame.render_widget(Paragraph::new(lines), row.text);
}
