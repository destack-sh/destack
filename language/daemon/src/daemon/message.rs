use std::path::PathBuf;

/// Severity classification for daemon messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonMessageKind {
    /// Informational messages for callers.
    Info,
    /// Warning messages that do not block progress.
    Warning,
    /// Error messages describing failed operations.
    Error,
}

/// Structured messages surfaced by daemon operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonMessage {
    /// The message severity.
    pub kind: DaemonMessageKind,
    /// Stable message code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional path related to this message.
    pub path: Option<PathBuf>,
}

impl DaemonMessage {
    /// Create a daemon message.
    pub fn new(
        kind: DaemonMessageKind,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            code: code.into(),
            message: message.into(),
            path: None,
        }
    }

    /// Create a daemon message with a path.
    pub fn with_path(
        kind: DaemonMessageKind,
        code: impl Into<String>,
        message: impl Into<String>,
        path: PathBuf,
    ) -> Self {
        Self {
            kind,
            code: code.into(),
            message: message.into(),
            path: Some(path),
        }
    }

    /// Return the message kind.
    pub fn kind(&self) -> DaemonMessageKind {
        self.kind
    }

    /// Return a stable code for this message.
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// Render the message for user facing output.
    pub fn render(&self) -> &str {
        self.message.as_str()
    }
}

impl std::fmt::Display for DaemonMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.render())
    }
}
