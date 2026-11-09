use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};
use ratatui::layout::Size;
use ratatui_image::Resize;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::Protocol;
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::config;
use crate::error::{Error, Result};

/// Thumbnails are about 15 KiB, so this keeps a few thousand.
const MAX_CACHE_BYTES: u64 = 64 * 1024 * 1024;

/// Shared by all downloads so thumbnails reuse open connections instead of shaking hands
/// again; TLS goes through macOS's Security framework and the system trust store.
static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    Agent::config_builder()
        .tls_config(tls)
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent()
});

/// Encodes the thumbnail of `id` for the terminal, scaled to `size` cells.
pub fn render(id: &str, size: Size, picker: &Picker) -> Result<Protocol> {
    let image = load(id)?;
    picker
        .new_protocol(image, size, Resize::Scale(Some(FilterType::Triangle)))
        .map_err(Error::from_display)
}

/// Deletes the oldest downloads once the cache outgrows [`MAX_CACHE_BYTES`].
pub fn prune_cache() {
    let Ok(dir) = fs::read_dir(cache_dir()) else {
        return;
    };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = dir
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            Some((meta.modified().ok()?, meta.len(), entry.path()))
        })
        .collect();
    files.sort_unstable_by_key(|(modified, ..)| Reverse(*modified));
    let mut total = 0;
    for (_, len, path) in files {
        total += len;
        if total > MAX_CACHE_BYTES {
            // One left behind is simply tried again next start.
            let _ = fs::remove_file(path);
        }
    }
}

fn load(id: &str) -> Result<DynamicImage> {
    let path = cache_path(id);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => {
            let bytes = download(id)?;
            // A missing cache only costs a re-download next time.
            let _ = store(&path, &bytes);
            bytes
        }
    };
    image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg).map_err(Error::from_display)
}

/// `mqdefault` is exactly 16:9, unlike the bigger sizes which are letterboxed 4:3.
fn download(id: &str) -> Result<Vec<u8>> {
    let url = format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg");
    let mut response = AGENT.get(&url).call().map_err(Error::from_display)?;
    response
        .body_mut()
        .read_to_vec()
        .map_err(Error::from_display)
}

/// Writes through a temporary file so a crash never leaves a truncated image behind.
fn store(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension("part");
    fs::write(&partial, bytes)?;
    fs::rename(partial, path)
}

fn cache_dir() -> PathBuf {
    config::cache_dir().join("thumbnails")
}

fn cache_path(id: &str) -> PathBuf {
    cache_dir().join(format!("{id}.jpg"))
}
