//! Screen geometry, shared by drawing and by key handling (which needs to know how many
//! cards fit in a row or column).

use ratatui::layout::{Constraint, Layout, Margin, Rect, Size};
use ratatui_image::FontSize;

pub struct Areas {
    pub header: Rect,
    pub chips: Rect,
    pub body: Rect,
    pub footer: Rect,
}

impl Areas {
    pub fn new(screen: Rect, with_chips: bool) -> Self {
        let chips_height = if with_chips { 2 } else { 0 };
        let [header, chips, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(chips_height),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(screen);
        let chips = Rect {
            height: chips.height.min(1),
            ..chips
        };
        Self {
            header,
            chips,
            body: body.inner(Margin::new(1, 0)),
            footer,
        }
    }
}

const MIN_CARD_WIDTH: u16 = 34;
const MAX_COLUMNS: u16 = 4;
/// Title (two lines), channel, and views.
const TEXT_ROWS: u16 = 4;
/// Every card reserves a border, drawn only around the selected one.
const BORDER: u16 = 2;

/// Card placement of the video grid.
#[derive(Clone, Copy)]
pub struct Grid {
    area: Rect,
    pub columns: usize,
    pub rows: usize,
    card: Size,
    pub thumbnail: Size,
}

impl Grid {
    pub fn new(area: Rect, font: FontSize) -> Self {
        let columns = (area.width / MIN_CARD_WIDTH).clamp(1, MAX_COLUMNS);
        let card_width = area.width / columns;
        let inner_width = card_width.saturating_sub(BORDER);
        let room = area.height.saturating_sub(TEXT_ROWS + BORDER).max(1);
        let thumbnail_height = picture_height(inner_width, font).clamp(1, room);
        let card_height = thumbnail_height + TEXT_ROWS + BORDER;
        Self {
            area,
            columns: usize::from(columns),
            rows: usize::from((area.height / card_height).max(1)),
            card: Size::new(card_width, card_height),
            thumbnail: Size::new(inner_width, thumbnail_height),
        }
    }

    /// Cards on fully visible rows; navigation keeps the selection among these.
    pub fn visible(&self) -> usize {
        self.columns * self.rows
    }

    /// Cards to draw: the full rows plus the partly visible row under them.
    pub fn drawn(&self) -> usize {
        let rows_on_screen = usize::from(self.area.height).div_ceil(usize::from(self.card.height));
        self.columns * rows_on_screen.max(self.rows)
    }

    /// The outer rectangle of the `slot`-th visible card, counted row by row.
    pub fn card(&self, slot: usize) -> Rect {
        let column = u16::try_from(slot % self.columns).unwrap_or(0);
        let row = u16::try_from(slot / self.columns).unwrap_or(0);
        Rect {
            x: self.area.x + column * self.card.width,
            y: self.area.y + row * self.card.height,
            width: self.card.width,
            height: self.card.height,
        }
        .intersection(self.area)
    }
}

/// Widest the watch page gets; longer lines are hard to read.
const PAGE_MAX_WIDTH: u16 = 120;
/// Narrowest the page gets; below that "Up next" is left out instead.
const PAGE_MIN_WIDTH: u16 = 64;
const UP_NEXT_MIN_WIDTH: u16 = 40;
const UP_NEXT_MAX_WIDTH: u16 = 80;
const COLUMN_GAP: u16 = 3;
/// Room for the focus bar left of each "Up next" row.
const GUTTER: u16 = 2;
/// The "Up next" heading and a blank line.
const HEADING_ROWS: u16 = 2;

/// The watch page, with "Up next" beside it when the terminal is wide enough.
pub struct WatchColumns {
    pub page: Rect,
    pub up_next: Option<UpNext>,
}

impl WatchColumns {
    pub fn new(area: Rect, font: FontSize) -> Self {
        if area.width < PAGE_MIN_WIDTH + COLUMN_GAP + UP_NEXT_MIN_WIDTH {
            let page = Rect {
                width: area.width.min(PAGE_MAX_WIDTH),
                ..area
            };
            return Self {
                page,
                up_next: None,
            };
        }
        let page_width = (area.width - COLUMN_GAP - UP_NEXT_MIN_WIDTH).min(PAGE_MAX_WIDTH);
        let side = Rect {
            x: area.x + page_width + COLUMN_GAP,
            width: (area.width - page_width - COLUMN_GAP).min(UP_NEXT_MAX_WIDTH),
            ..area
        };
        Self {
            page: Rect {
                width: page_width,
                ..area
            },
            up_next: Some(UpNext::new(side, font)),
        }
    }
}

/// The "Up next" column: a heading, then one row per video, thumbnail on the left.
#[derive(Clone, Copy)]
pub struct UpNext {
    pub heading: Rect,
    list: Rect,
    row_height: u16,
    pub thumbnail: Size,
}

/// One "Up next" row, split into its parts.
pub struct Row {
    pub gutter: Rect,
    pub picture: Rect,
    pub text: Rect,
}

impl UpNext {
    fn new(area: Rect, font: FontSize) -> Self {
        let [heading, list] =
            Layout::vertical([Constraint::Length(HEADING_ROWS), Constraint::Fill(1)]).areas(area);
        // Two fifths of the width, like the thumbnails next to a video on youtube.com.
        let width = area.width.saturating_sub(GUTTER) * 2 / 5;
        let height = picture_height(width, font).clamp(1, list.height.max(1));
        Self {
            heading: Rect {
                height: 1,
                ..heading
            },
            list,
            // A blank line between rows.
            row_height: height.max(TEXT_ROWS) + 1,
            thumbnail: Size::new(width, height),
        }
    }

    /// Rows fully on screen; scrolling keeps the selection among these.
    pub fn rows(&self) -> usize {
        usize::from(self.list.height / self.row_height).max(1)
    }

    /// Rows to draw: the full ones plus a partly visible one under them.
    pub fn drawn(&self) -> usize {
        usize::from(self.list.height).div_ceil(usize::from(self.row_height))
    }

    /// The `slot`-th row on screen.
    pub fn row(&self, slot: usize) -> Row {
        let offset = u16::try_from(slot)
            .unwrap_or(u16::MAX)
            .saturating_mul(self.row_height);
        let row = Rect {
            y: self.list.y.saturating_add(offset),
            height: self.row_height - 1,
            ..self.list
        }
        .intersection(self.list);
        let [gutter, picture, _, text] = Layout::horizontal([
            Constraint::Length(GUTTER),
            Constraint::Length(self.thumbnail.width),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(row);
        Row {
            gutter,
            picture: Rect {
                height: picture.height.min(self.thumbnail.height),
                ..picture
            },
            text,
        }
    }
}

/// Rows a 16:9 picture `width` cells wide needs; cells are usually twice as tall as wide.
fn picture_height(width: u16, font: FontSize) -> u16 {
    let pixels_high = u32::from(width) * u32::from(font.width) * 9 / 16;
    let rows = pixels_high / u32::from(font.height.max(1));
    u16::try_from(rows).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_cards_to_the_terminal() {
        let grid = Grid::new(Rect::new(0, 0, 160, 45), FontSize::new(10, 20));
        assert_eq!(grid.columns, 4);
        assert_eq!(grid.thumbnail, Size::new(38, 10));
        assert_eq!(grid.rows, 2);
        assert_eq!(grid.drawn(), 12);
        assert_eq!(grid.card(5), Rect::new(40, 16, 40, 16));
    }

    #[test]
    fn puts_up_next_beside_the_page_when_wide_enough() {
        let font = FontSize::new(10, 20);
        let wide = WatchColumns::new(Rect::new(1, 3, 218, 45), font);
        assert_eq!(wide.page.width, 120);
        let up_next = wide
            .up_next
            .map(|up_next| (up_next.heading, up_next.thumbnail));
        assert_eq!(up_next, Some((Rect::new(124, 3, 80, 1), Size::new(31, 8))));

        let narrow = WatchColumns::new(Rect::new(1, 3, 100, 45), font);
        assert_eq!(narrow.page.width, 100);
        assert!(narrow.up_next.is_none());
    }

    #[test]
    fn splits_up_next_rows() {
        let font = FontSize::new(10, 20);
        let up_next = WatchColumns::new(Rect::new(0, 0, 163, 32), font).up_next;
        // 40 wide: a 15x4 picture, rows of 5, under a 2-line heading.
        let counts = up_next.map(|up_next| (up_next.rows(), up_next.drawn()));
        assert_eq!(counts, Some((6, 6)));
        let row = up_next.map(|up_next| up_next.row(1));
        let parts = row.map(|row| (row.picture, row.text));
        assert_eq!(
            parts,
            Some((Rect::new(125, 7, 15, 4), Rect::new(141, 7, 22, 4)))
        );
    }
}
