use std::io::{BufRead, BufReader, Read};
use std::process::{Child, ChildStdout, Command, Stdio};

use crate::browser::Account;
use crate::error::{Error, Result};

/// Runs yt-dlp and returns its stdout. With an account, requests carry its Google
/// session; cookies are read fresh every time, so nothing secret is stored by us.
pub fn run(args: &[&str], account: Option<&Account>) -> Result<Vec<u8>> {
    let output = command(args, account)
        .output()
        .map_err(|err| Error::new(format!("cannot run yt-dlp: {err}")))?;

    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(failure(&output.stderr))
}

/// A yt-dlp whose stdout is read line by line while it is still working. It stops for
/// as long as nobody reads, and is killed when dropped.
pub struct Lines {
    child: Child,
    stdout: BufReader<ChildStdout>,
}

impl Lines {
    pub fn spawn(args: &[&str], account: Option<&Account>) -> Result<Self> {
        let mut child = command(args, account)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| Error::new(format!("cannot run yt-dlp: {err}")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::new("cannot read from yt-dlp"))?;
        Ok(Self {
            child,
            stdout: BufReader::new(stdout),
        })
    }

    /// Replaces `line` with the next one; `false` once yt-dlp finished without an error.
    pub fn next(&mut self, line: &mut String) -> Result<bool> {
        line.clear();
        if self.stdout.read_line(line)? > 0 {
            return Ok(true);
        }
        if self.child.wait()?.success() {
            return Ok(false);
        }
        let mut stderr = Vec::new();
        if let Some(mut pipe) = self.child.stderr.take() {
            // Whatever could be read still names the reason.
            let _ = pipe.read_to_end(&mut stderr);
        }
        Err(failure(&stderr))
    }
}

impl Drop for Lines {
    fn drop(&mut self) {
        // Fails only when yt-dlp already exited, which is what we want anyway.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn command(args: &[&str], account: Option<&Account>) -> Command {
    let mut command = Command::new("yt-dlp");
    command.args(["--no-warnings", "--no-progress"]);
    if let Some(account) = account {
        command.args(["--cookies-from-browser", &account.cookie_source()]);
    }
    command.args(args).stdin(Stdio::null());
    command
}

/// The last thing yt-dlp wrote to stderr: why it failed.
fn failure(stderr: &[u8]) -> Error {
    let stderr = String::from_utf8_lossy(stderr);
    let reason = stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("yt-dlp failed");
    Error::new(reason.trim_start_matches("ERROR: "))
}
