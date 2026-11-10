mod comments;
mod info;
mod page;
mod up_next;

use std::ops::Range;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;

use crate::app::{App, Column, Watch};
use crate::layout::WatchColumns;
use page::Page;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let columns = WatchColumns::new(area, app.font);
    if let (Some(up_next), Some(watch)) = (&columns.up_next, &app.watch) {
        up_next::render(frame, up_next, app, watch);
    }
    if let Some(watch) = app.watch.as_mut() {
        render_page(frame, columns.page, watch);
    }
}

fn render_page(frame: &mut Frame, area: Rect, watch: &mut Watch) {
    let focus = (watch.column == Column::Page).then_some(watch.focus);
    let mut page = Page::new(usize::from(area.width), focus);
    info::write(&mut page, watch);
    comments::write(&mut page, watch);

    watch.viewport = usize::from(area.height);
    watch.focus = watch.focus.min(page.items.len().saturating_sub(1));
    if let Some(item) = page.items.get(watch.focus) {
        watch.scroll = keep_visible(watch.scroll, item, watch.viewport);
    }
    let max_scroll = page.lines.len().saturating_sub(watch.viewport);
    watch.scroll = watch.scroll.min(max_scroll);
    watch.item_lines = page.items;

    let scroll = u16::try_from(watch.scroll).unwrap_or(u16::MAX);
    frame.render_widget(Paragraph::new(page.lines).scroll((scroll, 0)), area);
}

/// Shows the whole item when it fits, otherwise at least part of it.
fn keep_visible(scroll: usize, item: &Range<usize>, viewport: usize) -> usize {
    let bottom = scroll + viewport;
    if item.len() <= viewport {
        if item.start < scroll {
            item.start
        } else if item.end > bottom {
            item.end - viewport
        } else {
            scroll
        }
    } else if item.end <= scroll || item.start >= bottom {
        item.start
    } else {
        scroll
    }
}

#[cfg(test)]
mod tests {
    use super::keep_visible;

    #[test]
    fn scrolls_just_enough() {
        assert_eq!(keep_visible(0, &(5..8), 10), 0);
        assert_eq!(keep_visible(0, &(12..15), 10), 5);
        assert_eq!(keep_visible(20, &(12..15), 10), 12);
        assert_eq!(keep_visible(0, &(12..40), 10), 12);
        assert_eq!(keep_visible(15, &(12..40), 10), 15);
    }
}
