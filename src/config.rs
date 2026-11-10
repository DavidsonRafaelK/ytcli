use std::path::PathBuf;
use std::{env, fs, io};

use crate::browser::{self, Browser};

/// Settings that survive restarts, stored as `key=value` lines.
#[derive(Debug)]
pub struct Config {
    /// The browser whose Google session is used. `None` means signed out.
    pub browser: Option<&'static Browser>,
    /// Whether a related video plays when one ends and the queue is empty.
    pub autoplay: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            browser: None,
            autoplay: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut config = Self::default();
        let Ok(text) = fs::read_to_string(config_file()) else {
            return config;
        };
        for (key, value) in text.lines().filter_map(|line| line.split_once('=')) {
            match key.trim() {
                "browser" => config.browser = browser::find(value.trim()),
                "autoplay" => config.autoplay = value.trim() != "off",
                _ => {}
            }
        }
        config
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_file();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut body = String::new();
        if let Some(browser) = self.browser {
            body.push_str(&format!("browser={}\n", browser.id));
        }
        if !self.autoplay {
            body.push_str("autoplay=off\n");
        }
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
