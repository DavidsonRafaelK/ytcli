use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn width(text: &str) -> usize {
    text.width()
}

pub fn wrap(text: &str, width: usize) -> Vec<String> {
    textwrap::wrap(text, width.max(1))
        .into_iter()
        .map(|line| line.into_owned())
        .collect()
}

/// Wraps to at most `max_lines`, marking a cut with an ellipsis.
pub fn clamp(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut lines = wrap(text, width);
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            *last = ellipsize(&format!("{last} …"), width);
        }
    }
    lines
}

/// Cuts `text` to `width` columns, ending with `…` when something was dropped.
pub fn ellipsize(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    let mut used = 0;
    let mut out = String::new();
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w + 1 > width {
            break;
        }
        used += w;
        out.push(c);
    }
    out.push('…');
    out
}

/// Pads with spaces to exactly `width` columns (or cuts, if longer).
pub fn pad(text: &str, width: usize) -> String {
    let text = ellipsize(text, width);
    let fill = width.saturating_sub(text.width());
    format!("{text}{}", " ".repeat(fill))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ellipsizes_by_display_width() {
        assert_eq!(ellipsize("hello", 5), "hello");
        assert_eq!(ellipsize("hello world", 6), "hello…");
        assert_eq!(ellipsize("日本語です", 5), "日本…");
    }

    #[test]
    fn clamps_lines() {
        let lines = clamp("one two three four five six", 9, 2);
        assert_eq!(lines, ["one two", "three …"]);
    }
}
