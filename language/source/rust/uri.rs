use core::fmt;
use std::borrow::Borrow;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Uri(String);

impl Uri {
    /// Create a new URI from a string.
    pub fn from_string<T: Into<String>>(uri: T) -> Self {
        Self(uri.into())
    }

    /// Get the last segment of the URI.
    pub fn last_segment(&self) -> Option<&str> {
        self.0.split("/").last()
    }
}

impl FromStr for Uri {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl AsRef<str> for Uri {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&PathBuf> for Uri {
    fn from(path: &PathBuf) -> Self {
        Self(path.to_string_lossy().into_owned())
    }
}

impl Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Borrow<str> for Uri {
    fn borrow(&self) -> &str {
        &self.0
    }
}
