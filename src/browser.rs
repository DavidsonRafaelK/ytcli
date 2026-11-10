use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs, io};

use serde::Deserialize;

use crate::config;

/// A browser yt-dlp can borrow the Google session from.
#[derive(Debug, PartialEq, Eq)]
pub struct Browser {
    /// The name yt-dlp's `--cookies-from-browser` expects.
    pub id: &'static str,
    pub name: &'static str,
    mac_app: &'static str,
    /// For Chromium browsers, their data folder under `~/Library/Application Support`,
    /// which holds one folder per profile.
    profiles_dir: Option<&'static str>,
}

pub static ALL: [Browser; 8] = [
    Browser::new(
        "chrome",
        "Google Chrome",
        "Google Chrome",
        Some("Google/Chrome"),
    ),
    Browser::new("safari", "Safari", "Safari", None),
    Browser::new("firefox", "Firefox", "Firefox", None),
    Browser::new(
        "brave",
        "Brave",
        "Brave Browser",
        Some("BraveSoftware/Brave-Browser"),
    ),
    Browser::new(
        "edge",
        "Microsoft Edge",
        "Microsoft Edge",
        Some("Microsoft Edge"),
    ),
    Browser::new("chromium", "Chromium", "Chromium", Some("Chromium")),
    Browser::new("opera", "Opera", "Opera", None),
    Browser::new("vivaldi", "Vivaldi", "Vivaldi", Some("Vivaldi")),
];

const SIGN_IN_URL: &str = "https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2F";

/// A browser, and for Chromium browsers one of its profiles, whose Google session yt
/// borrows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub browser: &'static Browser,
    /// The profile's folder name, like `Default` or `Profile 2`.
    pub profile: Option<Box<str>>,
    /// What the browser calls the profile, and the Google account signed in to it.
    pub profile_name: Option<Box<str>>,
    pub email: Option<Box<str>>,
}

/// The parts of a Chromium `Local State` file that describe its profiles.
#[derive(Deserialize)]
struct LocalState {
    profile: Option<ProfileState>,
}

#[derive(Deserialize)]
struct ProfileState {
    last_used: Option<Box<str>>,
    #[serde(default)]
    info_cache: std::collections::BTreeMap<Box<str>, ProfileInfo>,
}

#[derive(Deserialize)]
struct ProfileInfo {
    name: Option<Box<str>>,
    user_name: Option<Box<str>>,
}

/// One entry per profile of every browser with an app bundle in `/Applications` or
/// `~/Applications`; every browser when none is found there.
pub fn accounts() -> Vec<Account> {
    let installed: Vec<_> = ALL
        .iter()
        .filter(|browser| browser.is_installed())
        .collect();
    let browsers = if installed.is_empty() {
        ALL.iter().collect()
    } else {
        installed
    };
    browsers.into_iter().flat_map(Browser::accounts).collect()
}

/// The account a config file names. Without a profile, the one the browser used last.
pub fn find(id: &str, profile: Option<&str>) -> Option<Account> {
    let browser = ALL.iter().find(|browser| browser.id == id)?;
    let state = browser.local_state();
    let profile = profile
        .map(Box::from)
        .or_else(|| state.as_ref().and_then(|state| state.last_used.clone()))
        .or_else(|| browser.profiles_dir.map(|_| "Default".into()));
    Some(browser.account(profile, state.as_ref()))
}

impl Browser {
    const fn new(
        id: &'static str,
        name: &'static str,
        mac_app: &'static str,
        profiles_dir: Option<&'static str>,
    ) -> Self {
        Self {
            id,
            name,
            mac_app,
            profiles_dir,
        }
    }

    fn is_installed(&self) -> bool {
        let bundle = format!("{}.app", self.mac_app);
        let home_apps = env::var_os("HOME").map(|home| Path::new(&home).join("Applications"));
        [Some(Path::new("/Applications").to_path_buf()), home_apps]
            .into_iter()
            .flatten()
            .any(|dir| dir.join(&bundle).exists())
    }

    /// Every profile of a Chromium browser, or the browser itself.
    fn accounts(&'static self) -> Vec<Account> {
        let state = self.local_state();
        let profiles: Vec<_> = state
            .as_ref()
            .map(|state| state.info_cache.keys().cloned().collect())
            .unwrap_or_default();
        if profiles.is_empty() {
            let profile = self.profiles_dir.map(|_| "Default".into());
            return vec![self.account(profile, state.as_ref())];
        }
        profiles
            .into_iter()
            .map(|profile| self.account(Some(profile), state.as_ref()))
            .collect()
    }

    fn account(&'static self, profile: Option<Box<str>>, state: Option<&ProfileState>) -> Account {
        let info = profile
            .as_deref()
            .and_then(|profile| state?.info_cache.get(profile));
        Account {
            browser: self,
            profile_name: info.and_then(|info| info.name.clone()),
            email: info
                .and_then(|info| info.user_name.clone())
                .filter(|email| !email.is_empty()),
            profile,
        }
    }

    fn local_state(&self) -> Option<ProfileState> {
        let path = self.data_dir()?.join("Local State");
        let state: LocalState = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        state.profile
    }

    fn data_dir(&self) -> Option<PathBuf> {
        let home = env::var_os("HOME")?;
        let dir = Path::new(&home).join("Library/Application Support");
        Some(dir.join(self.profiles_dir?))
    }
}

impl Account {
    /// "Google Chrome · Davidson", or just the browser.
    pub fn label(&self) -> String {
        match &self.profile_name {
            Some(profile) => format!("{} · {profile}", self.browser.name),
            None => self.browser.name.to_owned(),
        }
    }

    /// What yt-dlp's `--cookies-from-browser` gets. yt-dlp reads the newest `Cookies`
    /// file anywhere below the profile folder, and Chrome keeps more of those for its own
    /// features, without the YouTube session. So a profile is passed as a folder holding
    /// only a link to its real cookie database, which is still read fresh every time.
    pub fn cookie_source(&self) -> String {
        let link = self
            .profile
            .as_deref()
            .and_then(|profile| self.link_cookies(profile).ok());
        match (link, &self.profile) {
            (Some(dir), _) => format!("{}:{}", self.browser.id, dir.display()),
            (None, Some(profile)) => format!("{}:{profile}", self.browser.id),
            (None, None) => self.browser.id.to_owned(),
        }
    }

    fn link_cookies(&self, profile: &str) -> io::Result<PathBuf> {
        let profile_dir = self
            .browser
            .data_dir()
            .ok_or_else(|| io::Error::other("no profiles"))?
            .join(profile);
        // Newer Chrome versions moved the database into `Network`.
        let database = [
            profile_dir.join("Network/Cookies"),
            profile_dir.join("Cookies"),
        ]
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| io::Error::other("no cookie database"))?;

        let name = format!("{}-{}", self.browser.id, profile.replace(' ', "-"));
        let dir = config::cache_dir().join("cookies").join(name);
        let link = dir.join("Cookies");
        if fs::read_link(&link).ok().as_deref() != Some(database.as_path()) {
            fs::create_dir_all(&dir)?;
            let _ = fs::remove_file(&link);
            std::os::unix::fs::symlink(&database, &link)?;
        }
        Ok(dir)
    }

    /// Opens Google's sign-in page in this browser and profile; it lands on YouTube
    /// afterwards.
    pub fn open_sign_in(&self) -> io::Result<()> {
        let mut command = Command::new("open");
        match &self.profile {
            // A new instance hands its arguments over to the running browser, which
            // then opens the page in that profile.
            Some(profile) => command
                .args(["-n", "-a", self.browser.mac_app, "--args"])
                .args([
                    format!("--profile-directory={profile}"),
                    SIGN_IN_URL.to_owned(),
                ]),
            None => command.args(["-a", self.browser.mac_app, SIGN_IN_URL]),
        };
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(drop)
    }
}
