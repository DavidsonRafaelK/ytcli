use std::fmt;

/// A message meant for the user; every error in this app ends up in the status line.
#[derive(Debug, Clone)]
pub struct Error(Box<str>);

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(message: impl Into<Box<str>>) -> Self {
        Self(message.into())
    }

    pub fn from_display(err: impl fmt::Display) -> Self {
        Self::new(err.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::from_display(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::from_display(err)
    }
}
