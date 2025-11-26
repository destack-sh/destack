use std::path::{Path, PathBuf};

/// The final resolved path with optional `?query` and `#fragment`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The path without query and fragment (like `foo.js` in `foo.js?query#fragment`).
    pub path: PathBuf,
    /// Query `?query` (like `?foo` in `foo.js?foo`, including the `?`).
    pub query: Option<String>,
    /// Fragment `#query` (like `#foo` in `foo.js#foo`, including the `#`).
    pub fragment: Option<String>,
}

impl Resolution {
    /// Returns the path without query and fragment.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path without query and fragment.
    pub fn into_path_buf(self) -> PathBuf {
        self.path.clone()
    }

    /// Builds the full path with query and fragment.
    pub fn full_path(&self) -> PathBuf {
        let mut path = self.path.clone().into_os_string();
        if let Some(query) = &self.query {
            path.push(query);
        }
        if let Some(fragment) = &self.fragment {
            path.push(fragment);
        }
        PathBuf::from(path)
    }
}
