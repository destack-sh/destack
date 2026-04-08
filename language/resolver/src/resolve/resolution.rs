use std::path::{Path, PathBuf};

/// The final resolved path with optional `?query` and `#fragment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The path without query or fragment.
    pub path: PathBuf,
    /// The query suffix, including the leading `?`.
    pub query: Option<String>,
    /// The fragment suffix, including the leading `#`.
    pub fragment: Option<String>,
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

    /// Create one resolution with explicit query and fragment parts.
    pub(crate) fn with_parts(
        path: PathBuf,
        query: Option<String>,
        fragment: Option<String>,
    ) -> Self {
        Self {
            path,
            query,
            fragment,
        }
    }

    /// Override query and fragment when the new parts are present.
    pub(crate) fn override_parts(
        mut self,
        query: Option<String>,
        fragment: Option<String>,
    ) -> Self {
        if query.is_some() {
            self.query = query;
        }

        if fragment.is_some() {
            self.fragment = fragment;
        }

        self
    }

    /// Fill query and fragment only when they are still missing.
    pub(crate) fn fill_missing_parts(
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

    /// Return the path without query or fragment.
    pub fn into_path_buf(self) -> PathBuf {
        self.path.clone()
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
