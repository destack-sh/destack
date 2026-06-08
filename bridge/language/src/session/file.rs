use crate::bridge;

/// One editable file visible to a live session.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFile {
    /// Repository logical path.
    pub path: String,
}

impl SessionFile {
    /// Create one session file value.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}
