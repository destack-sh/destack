/// Failure while creating or operating one physical file watch.
#[derive(Debug)]
pub struct FileWatchError {
    /// The host watcher failure.
    source: notify::Error,
}

impl std::fmt::Display for FileWatchError {
    /// Format this file watch failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for FileWatchError {
    /// Return the host watcher failure.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl From<notify::Error> for FileWatchError {
    /// Convert one host watcher failure.
    fn from(error: notify::Error) -> Self {
        Self { source: error }
    }
}
