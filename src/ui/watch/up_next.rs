use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui_image::Image;

use crate::app::{App, Column, Loadable, Watch};
use crate::format;
use crate::layout::{Row, UpNext};
use crate::model::VideoCard;
use crate::ui::{card, text, theme};

/// The queue and then related videos beside the page, one row each: thumbnail, then
/// title, channel and views.
pub fn render(frame: &mut Frame, up_next: &UpNext, app: &App, watch: &Watch) {
    frame.render_widget(Line::styled("Up next", theme::bold()), up_next.heading);
    let autoplay = if app.config.autoplay {
        "Autoplay on"
    } else {
        "Autoplay off"
    };
    frame.render_widget(
        Paragraph::new(Line::styled(autoplay, theme::muted())).alignment(Alignment::Right),
        up_next.heading,
    );

    let cards = app.up_next_cards();
    if cards.is_empty() {
        let message = match &watch.related {
            Loadable::Loading => Line::styled("Loading…", theme::muted()),
            Loadable::Failed(err) => Line::styled(err.to_string(), theme::accent()),
            Loadable::Ready(_) => Line::styled("No related videos.", theme::muted()),
        };
        // Lined up with the titles of the rows.
        let area = up_next.row(0).picture;
        let area = Rect {
            width: up_next.heading.right().saturating_sub(area.x),
            ..area
        };
        frame.render_widget(Paragraph::new(message).wrap(Wrap { trim: true }), area);
        return;
    }

    let focused = watch.column == Column::UpNext;
    let next = app.next_up().map(|card| &card.id);
    let visible = cards.into_iter().enumerate().skip(watch.up_next_top);
    for (slot, (index, video)) in visible.take(up_next.drawn()).enumerate() {
        let label = if next == Some(&video.id) {
            Some("Next")
        } else if app.is_queued(&video.id) {
            Some("Queued")
        } else {
            None
        };
        let selected = focused && index == watch.up_next_selected;
        row(frame, &up_next.row(slot), video, selected, label, app);
    }
}

fn row(
    frame: &mut Frame,
    row: &Row,
    video: &VideoCard,
    selected: bool,
    label: Option<&'static str>,
    app: &App,
) {
    if selected {
        let bar = vec![Line::styled("▌", theme::accent()); usize::from(row.gutter.height)];
        frame.render_widget(Paragraph::new(bar), row.gutter);
    }

    match app.thumbnails.get(&video.id) {
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
    lines.extend(card::meta_line(video.views, label));
    frame.render_widget(Paragraph::new(lines), row.text);
}
