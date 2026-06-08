use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use destack_repository::Revision;
use destack_session::SourceUpdate;
use destack_source::{
    Diagnostic, FileId, FileWatchEvent, FileWatchEventKind, FileWatchRescanReason, FileWatchStatus,
    ModuleId,
};
use destack_workspace::{
    Error, FileChange, FileImage, FileUpdate, FileUpdateKind, Message, MessageKind, UpdateBatch,
};

use crate::{DaemonError, DaemonMessage, DaemonMessageKind, DaemonWorkspace, WatchBatch};

/// Summary of a daemon update.
#[derive(Debug, Clone)]
pub struct DaemonUpdate {
    /// Module affected by the update.
    pub module_id: Option<ModuleId>,
    /// File affected by the update.
    pub file_id: FileId,
    /// Current file image, when the file exists.
    pub file: Option<FileImage>,
    /// Kind of file update.
    pub kind: FileUpdateKind,
    /// Diagnostics produced by the update.
    pub diagnostics: Vec<Diagnostic>,
}

/// Result of applying an update through the daemon.
#[derive(Debug, Clone, Default)]
pub struct DaemonUpdateResult {
    /// File updates produced by the daemon.
    pub updates: Vec<DaemonUpdate>,
    /// Messages produced while applying the update.
    pub messages: Vec<DaemonMessage>,
}

/// Result of applying a source update through the daemon.
#[derive(Debug, Clone)]
pub struct DaemonSourceUpdateResult {
    /// Previous repository revision.
    pub before: Revision,
    /// Updated repository revision.
    pub after: Revision,
    /// File updates produced by the daemon.
    pub updates: Vec<DaemonUpdate>,
    /// Messages produced while applying the update.
    pub messages: Vec<DaemonMessage>,
}

impl DaemonUpdateResult {
    /// Return whether any files changed.
    pub fn updated(&self) -> bool {
        !self.updates.is_empty()
    }
}

impl DaemonWorkspace {
    /// Apply a text update and write it to disk.
    pub fn update_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileChange::Text { content }, true)
    }

    /// Apply a text update without writing to disk.
    pub fn update_memory_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileChange::Text { content }, false)
    }

    /// Mark a file as removed without touching the filesystem.
    pub fn remove_virtual_file(&self, path: &Path) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileChange::Removed, false)
    }

    /// Close one open file and restore filesystem truth.
    pub fn close_file(&self, path: &Path) -> Result<DaemonUpdateResult, DaemonError> {
        let workspace_result = self.workspace.close_file(path)?;

        Ok(daemon_update_result_from_workspace(workspace_result))
    }

    /// Apply a file update and optionally write to the filesystem.
    pub fn apply_file_update(
        &self,
        path: &Path,
        update: FileChange,
        write_to_disk: bool,
    ) -> Result<DaemonUpdateResult, DaemonError> {
        // write the update to disk when requested
        if write_to_disk {
            self.write_update_to_disk(path, &update)?;
        }

        // apply the update through workspace
        let workspace_result = self.workspace.apply_file(path, update)?;

        Ok(daemon_update_result_from_workspace(workspace_result))
    }

    /// Apply an atomic source update through the daemon.
    pub fn apply_source_update(
        &self,
        root: &Path,
        update: SourceUpdate,
    ) -> Result<DaemonSourceUpdateResult, DaemonError> {
        // apply the source batch through workspace
        let result = self.workspace.apply_source_update(root, update)?;
        let before = result.before;
        let after = result.after;
        let result = daemon_update_result(result.updates, result.messages);

        Ok(DaemonSourceUpdateResult {
            before,
            after,
            updates: result.updates,
            messages: result.messages,
        })
    }

    /// Apply a watch event through the daemon.
    pub fn apply_watch_event(&self, event: &FileWatchEvent) -> DaemonUpdateResult {
        // reuse the batch flow for single events
        let now = Instant::now();
        let batch = WatchBatch {
            events: vec![event.clone()],
            status: Vec::new(),
            started_at: now,
            ended_at: now,
            overflowed: matches!(event.kind, FileWatchEventKind::Overflow),
        };

        self.apply_watch_batch(&batch)
    }

    /// Apply a watch batch through the daemon.
    pub fn apply_watch_batch(&self, batch: &WatchBatch) -> DaemonUpdateResult {
        let mut result = DaemonUpdateResult::default();

        // apply watch statuses first
        let mut should_reload = false;
        for status in &batch.status {
            let status_result = self.handle_watch_status(status);
            should_reload |= status_result.should_reload;
            if let Some(message) = status_result.message {
                result.messages.push(message);
            }
        }

        // apply file events through workspace
        let has_overflow_event = batch
            .events
            .iter()
            .any(|event| matches!(event.kind, FileWatchEventKind::Overflow));
        match self.workspace.apply_watch_events(batch.events.clone()) {
            Ok(workspace_result) => {
                let daemon_result = daemon_update_result_from_workspace(workspace_result);
                result.updates.extend(daemon_result.updates);
                result.messages.extend(daemon_result.messages);
            }
            Err(error) => {
                result
                    .messages
                    .push(workspace_error_message("watch_apply_failed", &error));
            }
        }

        // handle overflow status batches without explicit overflow events
        if batch.overflowed && !has_overflow_event {
            should_reload = true;
        }

        // reload eagerly when status requests a full refresh
        if should_reload {
            match self.workspace.reload_all() {
                Ok(workspace_result) => {
                    let daemon_result = daemon_update_result_from_workspace(workspace_result);
                    result.updates.extend(daemon_result.updates);
                    result.messages.extend(daemon_result.messages);
                }
                Err(error) => {
                    result
                        .messages
                        .push(workspace_error_message("watch_reload_failed", &error));
                }
            }
        }

        result
    }

    /// Reload filesystem state for the provided roots.
    pub fn reload_roots(&self, roots: &[PathBuf]) -> DaemonUpdateResult {
        let result = self.workspace.reload_roots(roots);
        match result {
            Ok(result) => daemon_update_result_from_workspace(result),
            Err(error) => DaemonUpdateResult {
                updates: Vec::new(),
                messages: vec![DaemonMessage::new(
                    DaemonMessageKind::Warning,
                    "reload_filesystem_failed",
                    format!("watch: failed to reload filesystem state: {error}"),
                )],
            },
        }
    }

    /// Write a file update to disk before applying it.
    fn write_update_to_disk(&self, path: &Path, update: &FileChange) -> Result<(), DaemonError> {
        let parent = path.parent();
        if let Some(parent) = parent {
            self.repository
                .file_system()
                .create_dir_all(parent)
                .map_err(|error| DaemonError::FileWrite {
                    path: parent.to_path_buf(),
                    error,
                })?;
        }

        match update {
            FileChange::Text { content } => {
                self.repository
                    .file_system()
                    .write_string(path, content)
                    .map_err(|error| DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    })?;
            }
            FileChange::Bytes { content } => {
                self.repository
                    .file_system()
                    .write(path, content)
                    .map_err(|error| DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    })?;
            }
            FileChange::Removed => {
                if let Err(error) = self.repository.file_system().remove_file(path) {
                    if error.kind() != io::ErrorKind::NotFound {
                        return Err(DaemonError::FileWrite {
                            path: path.to_path_buf(),
                            error,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle watch status events.
    fn handle_watch_status(&self, status: &FileWatchStatus) -> WatchStatusResult {
        match status {
            FileWatchStatus::Error { message } => WatchStatusResult {
                should_reload: false,
                message: Some(DaemonMessage::new(
                    DaemonMessageKind::Warning,
                    "watch_status_error",
                    format!("watch: {message}"),
                )),
            },
            FileWatchStatus::RescanRequested { reason, .. } => WatchStatusResult {
                should_reload: true,
                message: Some(DaemonMessage::new(
                    DaemonMessageKind::Info,
                    "watch_reload_requested",
                    watch_reload_requested_message(reason),
                )),
            },
            FileWatchStatus::Ready { .. } => WatchStatusResult {
                should_reload: false,
                message: None,
            },
            FileWatchStatus::Stopped => WatchStatusResult {
                should_reload: false,
                message: None,
            },
        }
    }
}

/// Status handling result.
#[derive(Debug, Clone, Default)]
struct WatchStatusResult {
    /// Whether the status requires a reload.
    should_reload: bool,
    /// Optional surfaced message.
    message: Option<DaemonMessage>,
}

/// Convert a workspace update result into daemon shape.
fn daemon_update_result_from_workspace(result: UpdateBatch) -> DaemonUpdateResult {
    daemon_update_result(result.updates, result.messages)
}

/// Convert workspace updates and messages into daemon shape.
fn daemon_update_result(updates: Vec<FileUpdate>, messages: Vec<Message>) -> DaemonUpdateResult {
    DaemonUpdateResult {
        updates: updates
            .into_iter()
            .map(daemon_update_from_workspace)
            .collect(),
        messages: daemon_messages_from_workspace(messages),
    }
}

/// Convert workspace messages to daemon messages.
fn daemon_messages_from_workspace(messages: Vec<Message>) -> Vec<DaemonMessage> {
    messages
        .into_iter()
        .map(|message| {
            let kind = match message.kind {
                MessageKind::Info => DaemonMessageKind::Info,
                MessageKind::Warning => DaemonMessageKind::Warning,
                MessageKind::Error => DaemonMessageKind::Error,
            };

            DaemonMessage::new(kind, message.code, message.message)
        })
        .collect()
}

/// Build a daemon message for workspace failures.
fn workspace_error_message(code: &str, error: &Error) -> DaemonMessage {
    DaemonMessage::new(DaemonMessageKind::Error, code, error.to_string())
}

/// Build a watch reload requested message.
fn watch_reload_requested_message(reason: &FileWatchRescanReason) -> &'static str {
    match reason {
        FileWatchRescanReason::Startup => "watch: filesystem reload requested at startup",
        FileWatchRescanReason::Overflow => "watch: filesystem reload requested after overflow",
        FileWatchRescanReason::Manual => "watch: filesystem reload requested",
        FileWatchRescanReason::Update => "watch: filesystem reload requested after update",
    }
}

/// Convert a workspace update into daemon shape.
fn daemon_update_from_workspace(update: FileUpdate) -> DaemonUpdate {
    DaemonUpdate {
        module_id: update.module_id,
        file_id: update.file_id,
        file: update.file,
        kind: update.kind,
        diagnostics: update.diagnostics,
    }
}
