use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::page::Page;
use crate::app::{Loadable, Watch};
use crate::format;
use crate::model::{Comment, Thread};
use crate::ui::{text, theme};

const REPLY_INDENT: usize = 4;

/// The comment count, then one item per thread.
pub fn write(page: &mut Page, watch: &Watch) {
    page.blank();
    let heading = match watch
        .details
        .ready()
        .and_then(|details| details.comment_count)
    {
        Some(count) => format!("{} Comments", format::compact(count)),
        None => "Comments".into(),
    };
    page.push_plain(Line::styled(heading, theme::bold()));
    page.blank();

    let threads = match &watch.comments {
        Loadable::Loading => {
            return page.push_plain(Line::styled("Loading comments…", theme::muted()));
        }
        Loadable::Failed(err) => {
            return page.push_plain(Line::styled(err.to_string(), theme::accent()));
        }
        Loadable::Ready(threads) => threads,
    };
    if threads.is_empty() {
        page.push_plain(Line::styled("No comments to show.", theme::muted()));
    }
    for (index, thread) in threads.iter().enumerate() {
        let open = watch.open_threads.get(index).copied().unwrap_or(false);
        page.begin_item();
        write_thread(page, thread, open);
        page.end_item();
        page.blank();
    }
}

fn write_thread(page: &mut Page, thread: &Thread, open: bool) {
    write_comment(page, &thread.root, 0);

    let mut actions = vec![Span::raw(likes(&thread.root))];
    if !thread.replies.is_empty() {
        let action = if open { "Hide" } else { "Show" };
        let count = thread.replies.len();
        let noun = if count == 1 { "reply" } else { "replies" };
        actions.push(Span::styled(
            format!("   {action} {count} {noun}"),
            Style::new().fg(theme::FOCUS).bold(),
        ));
    }
    page.push(Line::from(actions));

    if open {
        for reply in &thread.replies {
            page.push(Line::default());
            write_comment(page, reply, REPLY_INDENT);
            page.push(Line::raw(format!(
                "{}{}",
                " ".repeat(REPLY_INDENT),
                likes(reply)
            )));
        }
    }
}

fn write_comment(page: &mut Page, comment: &Comment, indent: usize) {
    let pad = " ".repeat(indent);
    if comment.pinned {
        page.push(Line::styled(format!("{pad}Pinned"), theme::muted()));
    }
    let author_style = if comment.by_uploader {
        Style::new().bg(theme::SURFACE).bold()
    } else {
        theme::bold()
    };
    page.push(Line::from(vec![
        Span::raw(pad.clone()),
        Span::styled(comment.author.to_string(), author_style),
        Span::styled(format!("  {}", comment.age), theme::muted()),
    ]));
    let width = page.width().saturating_sub(indent);
    for line in text::wrap(&comment.text, width) {
        page.push(Line::raw(format!("{pad}{line}")));
    }
}

fn likes(comment: &Comment) -> String {
    match comment.likes.filter(|likes| *likes > 0) {
        Some(likes) => format!("{} likes", format::compact(likes)),
        None => "0 likes".into(),
    }
}
