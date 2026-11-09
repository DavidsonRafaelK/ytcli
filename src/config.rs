use std::path::PathBuf;
use std::{env, fs, io};

use crate::browser::{self, Browser};

/// Settings that survive restarts, stored as `key=value` lines.
#[derive(Debug, Default)]
pub struct Config {
    /// The browser whose Google session is used. `None` means signed out.
    pub browser: Option<&'static Browser>,
}

impl Config {
    pub fn load() -> Self {
        let Ok(text) = fs::read_to_string(config_file()) else {
            return Self::default();
        };
        let browser = text
            .lines()
            .filter_map(|line| line.split_once('='))
            .find(|(key, _)| key.trim() == "browser")
            .and_then(|(_, value)| browser::find(value.trim()));
        Self { browser }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_file();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let body = self
            .browser
            .map(|browser| format!("browser={}\n", browser.id))
            .unwrap_or_default();
        fs::write(path, body)
    }
}

/// `~/Library/Caches/yt`, which macOS may empty when the disk runs low.
pub fn cache_dir() -> PathBuf {
    library("Caches").join("yt")
}

fn config_file() -> PathBuf {
    library("Application Support").join("yt").join("config")
}

fn library(folder: &str) -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Library")
        .join(folder)
}
