use std::path::PathBuf;

use destack_source::{FileId, FileWatchRescanReason};

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
#[derive(Debug, Clone)]
pub enum DaemonMessage {
    /// Report that watch overflow requires a rescan.
    WatchOverflowRescan,
    /// Report that a watch removal failed.
    WatchRemoveFailed {
        /// The affected path.
        path: PathBuf,
        /// The failure detail.
        error: String,
    },
    /// Report that a watch read failed.
    WatchReadFailed {
        /// The affected path.
        path: PathBuf,
        /// The failure detail.
        error: String,
    },
    /// Report that a watch update failed.
    WatchUpdateFailed {
        /// The affected path.
        path: PathBuf,
        /// The failure detail.
        error: String,
    },
    /// Report that a rescan read failed.
    RescanReadFailed {
        /// The affected path.
        path: PathBuf,
        /// The failure detail.
        error: String,
    },
    /// Report that a rescan invalidation failed.
    RescanInvalidationFailed {
        /// The affected file id.
        file_id: FileId,
        /// The failure detail.
        error: String,
    },
    /// Report that a watcher status contained an error.
    WatchStatusError {
        /// The watcher error message.
        message: String,
    },
    /// Report that the watcher requested a rescan.
    WatchRescanRequested {
        /// The reason for the rescan request.
        reason: FileWatchRescanReason,
    },
    /// Report that a workspace config reload failed.
    ConfigReloadWorkspaceFailed {
        /// The failure detail.
        error: String,
    },
    /// Report that a package dsconfig reload failed.
    ConfigReloadPackageFailed {
        /// The package path.
        path: PathBuf,
        /// The failure detail.
        error: String,
    },
    /// Report that a tsconfig reload failed.
    ConfigReloadTsconfigFailed {
        /// The tsconfig path.
        path: PathBuf,
        /// The failure detail.
        error: String,
    },
}

impl DaemonMessage {
    /// Return the message kind.
    pub fn kind(&self) -> DaemonMessageKind {
        match self {
            DaemonMessage::WatchOverflowRescan => DaemonMessageKind::Warning,
            DaemonMessage::WatchRemoveFailed { .. } => DaemonMessageKind::Warning,
            DaemonMessage::WatchReadFailed { .. } => DaemonMessageKind::Warning,
            DaemonMessage::WatchUpdateFailed { .. } => DaemonMessageKind::Warning,
            DaemonMessage::RescanReadFailed { .. } => DaemonMessageKind::Warning,
            DaemonMessage::RescanInvalidationFailed { .. } => DaemonMessageKind::Warning,
            DaemonMessage::WatchStatusError { .. } => DaemonMessageKind::Warning,
            DaemonMessage::WatchRescanRequested { .. } => DaemonMessageKind::Info,
            DaemonMessage::ConfigReloadWorkspaceFailed { .. } => DaemonMessageKind::Warning,
            DaemonMessage::ConfigReloadPackageFailed { .. } => DaemonMessageKind::Warning,
            DaemonMessage::ConfigReloadTsconfigFailed { .. } => DaemonMessageKind::Warning,
        }
    }

    /// Return a stable code for this message.
    pub fn code(&self) -> &'static str {
        match self {
            DaemonMessage::WatchOverflowRescan => "watch_overflow_rescan",
            DaemonMessage::WatchRemoveFailed { .. } => "watch_remove_failed",
            DaemonMessage::WatchReadFailed { .. } => "watch_read_failed",
            DaemonMessage::WatchUpdateFailed { .. } => "watch_update_failed",
            DaemonMessage::RescanReadFailed { .. } => "rescan_read_failed",
            DaemonMessage::RescanInvalidationFailed { .. } => "rescan_invalidation_failed",
            DaemonMessage::WatchStatusError { .. } => "watch_status_error",
            DaemonMessage::WatchRescanRequested { .. } => "watch_rescan_requested",
            DaemonMessage::ConfigReloadWorkspaceFailed { .. } => "config_reload_workspace_failed",
            DaemonMessage::ConfigReloadPackageFailed { .. } => "config_reload_package_failed",
            DaemonMessage::ConfigReloadTsconfigFailed { .. } => "config_reload_tsconfig_failed",
        }
    }

    /// Render the message for user facing output.
    pub fn render(&self) -> String {
        match self {
            DaemonMessage::WatchOverflowRescan => {
                "watch: rescan required after overflow".to_string()
            }
            DaemonMessage::WatchRemoveFailed { path, error } => {
                format!("watch: failed to remove {}: {error}", path.display())
            }
            DaemonMessage::WatchReadFailed { path, error } => {
                format!("watch: failed to read {}: {error}", path.display())
            }
            DaemonMessage::WatchUpdateFailed { path, error } => {
                format!("watch: failed to update {}: {error}", path.display())
            }
            DaemonMessage::RescanReadFailed { path, error } => {
                format!("watch: failed to read {}: {error}", path.display())
            }
            DaemonMessage::RescanInvalidationFailed { file_id, error } => {
                format!("watch: failed to rescan {file_id:?}: {error}")
            }
            DaemonMessage::WatchStatusError { message } => format!("watch: {message}"),
            DaemonMessage::WatchRescanRequested { reason } => {
                let message = match reason {
                    FileWatchRescanReason::Startup => "watch: rescan requested at startup",
                    FileWatchRescanReason::Overflow => "watch: rescan requested after overflow",
                    FileWatchRescanReason::Manual => "watch: rescan requested",
                    FileWatchRescanReason::Update => "watch: rescan requested after update",
                };
                message.to_string()
            }
            DaemonMessage::ConfigReloadWorkspaceFailed { error } => {
                format!("watch: failed to reload workspace dsconfig: {error}")
            }
            DaemonMessage::ConfigReloadPackageFailed { path, error } => {
                format!("watch: failed to reload dsconfig for {path:?}: {error}")
            }
            DaemonMessage::ConfigReloadTsconfigFailed { path, error } => {
                format!("watch: failed to reload tsconfig {path:?}: {error}")
            }
        }
    }
}

impl std::fmt::Display for DaemonMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.render())
    }
}
