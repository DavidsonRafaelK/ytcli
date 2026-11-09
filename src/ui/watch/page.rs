use std::ops::Range;

use ratatui::text::{Line, Span};

use crate::ui::theme;

const GUTTER: usize = 2;

/// Lines of the watch page, grouped into focusable items. Lines of the focused item get a
/// bar in the gutter; with no `focus` none do.
pub struct Page {
    pub lines: Vec<Line<'static>>,
    pub items: Vec<Range<usize>>,
    width: usize,
    focus: Option<usize>,
    open_item: Option<usize>,
}

impl Page {
    pub fn new(width: usize, focus: Option<usize>) -> Self {
        Self {
            lines: Vec::new(),
            items: Vec::new(),
            width,
            focus,
            open_item: None,
        }
    }

    /// Columns available after the gutter.
    pub fn width(&self) -> usize {
        self.width.saturating_sub(GUTTER)
    }

    pub fn begin_item(&mut self) {
        self.open_item = Some(self.items.len());
        self.items.push(self.lines.len()..self.lines.len());
    }

    pub fn end_item(&mut self) {
        let end = self.lines.len();
        if let Some(range) = self
            .open_item
            .take()
            .and_then(|item| self.items.get_mut(item))
        {
            range.end = end;
        }
    }

    /// Adds a line that shows the focus bar when its item is focused.
    pub fn push(&mut self, line: Line<'static>) {
        let focused = self.open_item.is_some() && self.open_item == self.focus;
        self.push_with_gutter(line, focused);
    }

    /// Adds a line that never shows the focus bar, like the title above the description.
    pub fn push_plain(&mut self, line: Line<'static>) {
        self.push_with_gutter(line, false);
    }

    pub fn blank(&mut self) {
        self.lines.push(Line::default());
    }

    fn push_with_gutter(&mut self, mut line: Line<'static>, focused: bool) {
        let gutter = if focused {
            Span::styled("▌ ", theme::accent())
        } else {
            Span::raw("  ")
        };
        line.spans.insert(0, gutter);
        self.lines.push(line);
    }
}
