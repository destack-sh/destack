use std::path::{Path, PathBuf};

/// The final resolved path with optional `?query` and `#fragment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The path without query or fragment.
    path: PathBuf,
    /// The query suffix, including the leading `?`.
    query: Option<String>,
    /// The fragment suffix, including the leading `#`.
    fragment: Option<String>,
}

impl Resolution {
    /// Create one resolution for a plain filesystem path.
    pub(crate) fn path_only(path: PathBuf) -> Self {
        Self {
            path,
            query: None,
            fragment: None,
        }
    }

    /// Create one resolution with explicit path, query, and fragment.
    pub(crate) fn new(path: PathBuf, query: Option<String>, fragment: Option<String>) -> Self {
        Self {
            path,
            query,
            fragment,
        }
    }

    /// Fill query and fragment only when they are still missing.
    pub(crate) fn fill_missing_suffixes(
        mut self,
        query: Option<String>,
        fragment: Option<String>,
    ) -> Self {
        if self.query.is_none() {
            self.query = query;
        }

        if self.fragment.is_none() {
            self.fragment = fragment;
        }

        self
    }

    /// Return the path without query or fragment.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the query suffix.
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Return the fragment suffix.
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Return the path without query or fragment.
    pub fn into_path_buf(self) -> PathBuf {
        self.path
    }

    /// Consume this resolution into its path, query, and fragment.
    pub(crate) fn into_components(self) -> (PathBuf, Option<String>, Option<String>) {
        (self.path, self.query, self.fragment)
    }

    /// Build the full path with query and fragment.
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
