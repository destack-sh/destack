use core::fmt;
use std::borrow::Borrow;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A generic URI.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Uri(String);

impl Uri {
    /// Create a new URI from a path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        Self(path.as_ref().to_string_lossy().into_owned())
    }

    /// Create a new URI and a name from a path.
    pub fn from_path_with_name<P: AsRef<Path>>(path: P) -> (String, Self) {
        let path = path.as_ref();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let uri = Self(path.to_string_lossy().into_owned());
        (name, uri)
    }

    /// Create a canonical URI from one logical repository path.
    /// Workspace paths render as `file://`; mounted dependency paths as `mount://`.
    pub fn logical(logical_path: impl AsRef<str>) -> Self {
        let logical_path = logical_path.as_ref().replace('\\', "/");
        match logical_path.strip_prefix("mount:") {
            Some(mounted) => Self(format!("mount://{mounted}")),
            None => Self(format!("file://{logical_path}")),
        }
    }

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
        let Some(extension_start) = self.extension_start() else {
            return self.clone();
        };

        Self(self.0[..extension_start].to_string())
    }

    /// Append or replace the extension of the URI.
    pub fn with_extension(&self, extension: &str) -> Self {
        let extension = extension.trim_start_matches('.');
        let Some(extension_start) = self.extension_start() else {
            return Self(format!("{}.{}", self.0, extension));
        };

        Self(format!("{}.{}", &self.0[..extension_start], extension))
    }

    /// Return the byte index where the final path segment extension begins.
    fn extension_start(&self) -> Option<usize> {
        let separator_start = self
            .0
            .rfind(['/', '\\'])
            .map(|index| index + 1)
            .unwrap_or(0);
        let segment = &self.0[separator_start..];
        let dot = segment.rfind('.')?;

        if dot == 0 {
            return None;
        }

        Some(separator_start + dot)
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

impl From<PathBuf> for Uri {
    fn from(path: PathBuf) -> Self {
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
    pub fn to_path(&self) -> Option<&Path> {
        let path = Path::new(&self.0);
        Some(path)
    }

    /// Convert a URI to a PathBuf.
    pub fn to_path_buf(&self) -> Option<PathBuf> {
        let path = Path::new(&self.0);
        Some(path.to_path_buf())
    }

    /// Convert a file Path to a URI.
    pub fn from_file_path<A: AsRef<Path>>(path: A) -> Self {
        Self(path.as_ref().to_string_lossy().into_owned())
    }
}
