use std::collections::HashMap;

use ratatui::layout::Size;
use ratatui_image::protocol::Protocol;

use crate::error::Result;

enum Entry {
    Loading(Size),
    Ready(Size, Protocol),
    Failed,
}

/// Encoded thumbnails, at most one size per video.
#[derive(Default)]
pub struct Thumbnails {
    entries: HashMap<Box<str>, Entry>,
}

/// Encoded images are large (a full RGBA copy for kitty), so only a few screens are kept.
const MAX_ENTRIES: usize = 96;

impl Thumbnails {
    /// Whether `id` still has to be rendered at `size`; if so it is marked as on its way.
    pub fn claim(&mut self, id: &str, size: Size) -> bool {
        match self.entries.get(id) {
            Some(Entry::Loading(have) | Entry::Ready(have, _)) if *have == size => false,
            Some(Entry::Failed) => false,
            _ => {
                self.entries.insert(id.into(), Entry::Loading(size));
                true
            }
        }
    }

    /// Stores a result unless it was overtaken by a request for another size.
    pub fn receive(&mut self, id: Box<str>, size: Size, result: Result<Protocol>) {
        if !matches!(self.entries.get(&id), Some(Entry::Loading(wanted)) if *wanted == size) {
            return;
        }
        let entry = match result {
            Ok(image) => Entry::Ready(size, image),
            Err(_) => Entry::Failed,
        };
        self.entries.insert(id, entry);
    }

    pub fn get(&self, id: &str) -> Option<&Protocol> {
        match self.entries.get(id) {
            Some(Entry::Ready(_, image)) => Some(image),
            _ => None,
        }
    }

    /// Drops everything not in `keep` once the cache grows too big.
    pub fn trim<'a>(&mut self, keep: impl Iterator<Item = &'a str> + Clone) {
        if self.entries.len() > MAX_ENTRIES {
            self.entries
                .retain(|id, _| keep.clone().any(|kept| kept == &**id));
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
