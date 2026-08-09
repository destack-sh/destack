/// Workspace-local watch subscription identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WatchId(u64);

impl WatchId {
    /// Create one workspace-local watch identifier.
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for WatchId {
    /// Format the numeric watch identifier.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
