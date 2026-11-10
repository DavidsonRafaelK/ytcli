use ratatui::style::{Color, Modifier, Style};

pub const ACCENT: Color = Color::Rgb(255, 0, 51);
/// Chips, the description box, thumbnail placeholders.
pub const SURFACE: Color = Color::Rgb(39, 39, 39);
pub const MUTED: Color = Color::Rgb(170, 170, 170);
pub const FOCUS: Color = Color::Rgb(62, 166, 255);

pub const fn muted() -> Style {
    Style::new().fg(MUTED)
}

pub const fn bold() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

/// Labels such as "Queued" that stand out without looking like an error.
pub const fn highlight() -> Style {
    Style::new().fg(FOCUS).add_modifier(Modifier::BOLD)
}

pub const fn accent() -> Style {
    Style::new().fg(ACCENT)
}
