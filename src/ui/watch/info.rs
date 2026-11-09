use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::page::Page;
use crate::app::{Loadable, Watch};
use crate::format;
use crate::model::VideoDetails;
use crate::ui::{text, theme};

/// Lines of a collapsed description, like youtube.com's "...more".
const COLLAPSED_LINES: usize = 3;

/// Title, channel row and description box: the page's first item.
pub fn write(page: &mut Page, watch: &Watch) {
    let details = watch.details.ready();
    let width = page.width();
    page.begin_item();

    let title = details.map_or(&*watch.card.title, |details| &*details.title);
    for line in text::wrap(title, width) {
        page.push_plain(Line::styled(line, theme::bold()));
    }
    page.blank();
    page.push_plain(channel_row(watch, details, width));
    page.blank();

    match &watch.details {
        Loadable::Loading => page.push(Line::styled("Loading details…", theme::muted())),
        Loadable::Failed(err) => page.push(Line::styled(err.to_string(), theme::accent())),
        Loadable::Ready(details) => description(page, details, watch.description_open),
    }
    page.end_item();
}

fn channel_row(watch: &Watch, details: Option<&VideoDetails>, width: usize) -> Line<'static> {
    let channel = details.map_or(&*watch.card.channel, |details| &*details.channel);
    let mut left = vec![Span::styled(channel.to_owned(), theme::bold())];
    if details.is_some_and(|details| details.verified) {
        left.push(Span::styled(" (verified)", theme::muted()));
    }
    if let Some(subscribers) = details.and_then(|details| details.subscribers) {
        let label = format!("   {} subscribers", format::compact(subscribers));
        left.push(Span::styled(label, theme::muted()));
    }

    let likes = details
        .and_then(|details| details.likes)
        .map(|likes| format!("{} likes   ", format::compact(likes)))
        .unwrap_or_default();
    let right = Span::styled(format!("{likes}Playing in mpv"), theme::muted());

    let used: usize = left.iter().map(|span| text::width(&span.content)).sum();
    let gap = width
        .saturating_sub(used + text::width(&right.content))
        .max(2);
    left.push(Span::raw(" ".repeat(gap)));
    left.push(right);
    Line::from(left)
}

/// A shaded box: views and age, then the description, collapsed unless `open`.
fn description(page: &mut Page, details: &VideoDetails, open: bool) {
    let width = page.width();
    let inner = width.saturating_sub(2);
    let shaded = Style::new().bg(theme::SURFACE);

    let mut stats = Vec::new();
    if let Some(views) = details.views {
        stats.push(format::views(views));
    }
    if let Some(uploaded) = details.uploaded {
        stats.push(format::ago(uploaded));
    }
    page.push(Line::styled(
        text::pad(&format!(" {}", stats.join("  ")), width),
        shaded.bold(),
    ));

    let body = text::wrap(&details.description, inner);
    let long = body.len() > COLLAPSED_LINES;
    let shown = if open { body.len() } else { COLLAPSED_LINES };
    for line in body.iter().take(shown) {
        page.push(Line::styled(text::pad(&format!(" {line}"), width), shaded));
    }
    if long {
        let toggle = if open { " Show less" } else { " ...more" };
        page.push(Line::styled(text::pad(toggle, width), shaded.bold()));
    }
}
