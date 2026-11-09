use std::process::{Command, Stdio};

use crate::browser::Browser;
use crate::error::{Error, Result};

/// Runs yt-dlp and returns its stdout. With a browser, requests carry that browser's
/// Google session; cookies are read fresh every time, so nothing secret is stored by us.
pub fn run(args: &[&str], browser: Option<&Browser>) -> Result<Vec<u8>> {
    let mut command = Command::new("yt-dlp");
    command.args(["--no-warnings", "--no-progress"]);
    if let Some(browser) = browser {
        command.args(["--cookies-from-browser", browser.id]);
    }
    let output = command
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|err| Error::new(format!("cannot run yt-dlp: {err}")))?;

    if output.status.success() {
        return Ok(output.stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let reason = stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("yt-dlp failed");
    Err(Error::new(reason.trim_start_matches("ERROR: ")))
}
