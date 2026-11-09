use std::path::Path;
use std::process::{Command, Stdio};
use std::{env, io};

/// A browser yt-dlp can borrow the Google session from.
#[derive(Debug, PartialEq, Eq)]
pub struct Browser {
    /// The name yt-dlp's `--cookies-from-browser` expects.
    pub id: &'static str,
    pub name: &'static str,
    mac_app: &'static str,
}

pub static ALL: [Browser; 8] = [
    Browser::new("chrome", "Google Chrome", "Google Chrome"),
    Browser::new("safari", "Safari", "Safari"),
    Browser::new("firefox", "Firefox", "Firefox"),
    Browser::new("brave", "Brave", "Brave Browser"),
    Browser::new("edge", "Microsoft Edge", "Microsoft Edge"),
    Browser::new("chromium", "Chromium", "Chromium"),
    Browser::new("opera", "Opera", "Opera"),
    Browser::new("vivaldi", "Vivaldi", "Vivaldi"),
];

const SIGN_IN_URL: &str = "https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2F";

pub fn find(id: &str) -> Option<&'static Browser> {
    ALL.iter().find(|browser| browser.id == id)
}

/// Browsers with an app bundle in `/Applications` or `~/Applications`, or all of them
/// when none is found there.
pub fn installed() -> Vec<&'static Browser> {
    let found: Vec<_> = ALL
        .iter()
        .filter(|browser| browser.is_installed())
        .collect();
    if found.is_empty() {
        ALL.iter().collect()
    } else {
        found
    }
}

impl Browser {
    const fn new(id: &'static str, name: &'static str, mac_app: &'static str) -> Self {
        Self { id, name, mac_app }
    }

    fn is_installed(&self) -> bool {
        let bundle = format!("{}.app", self.mac_app);
        let home_apps = env::var_os("HOME").map(|home| Path::new(&home).join("Applications"));
        [Some(Path::new("/Applications").to_path_buf()), home_apps]
            .into_iter()
            .flatten()
            .any(|dir| dir.join(&bundle).exists())
    }

    /// Opens Google's sign-in page in this browser; it lands on YouTube afterwards.
    pub fn open_sign_in(&self) -> io::Result<()> {
        Command::new("open")
            .args(["-a", self.mac_app, SIGN_IN_URL])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(drop)
    }
}
