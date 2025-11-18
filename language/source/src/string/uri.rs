use core::fmt;
use std::borrow::Borrow;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// A generic URI.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
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

    /// Check if a URI starts with another URI.
    pub fn starts_with(&self, other: &Uri) -> bool {
        self.0.starts_with(&other.0)
    }

    /// Get the URI without the extension.
    pub fn without_extension(&self) -> Self {
        // trim extension if exists
        if self.0.contains(".") {
            let without_extension = self.0.split(".").next().unwrap();
            Self(without_extension.to_string())
        }
        // no extension
        else {
            self.clone()
        }
    }

    /// Append or replace the extension of the URI.
    pub fn with_extension(&self, extension: &str) -> Self {
        let extension = extension.trim_start_matches("."); // trim leading dot if any
        if self.0.contains(".") {
            let without_extension = format!("{}.{}", self.0.split(".").next().unwrap(), extension);
            Self(without_extension)
        } else {
            Self(format!("{}.{}", self.0, extension))
        }
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

impl Uri {
    /// Convert a URI to a Path.
    pub fn to_file_path(&self) -> Option<&Path> {
        let path = Path::new(&self.0);
        Some(path)
    }

    /// Convert a file Path to a URI.
    pub fn from_file_path<A: AsRef<Path>>(path: A) -> Self {
        Self(path.as_ref().to_string_lossy().into_owned())
    }
}
