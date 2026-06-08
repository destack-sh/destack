use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use destack_session::SourceUpdate;
use destack_source::{FileWatchEvent, FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};
use destack_workspace::{Error, FileChange, Message, MessageKind, SourceUpdateResult, UpdateBatch};

use crate::{DaemonError, WatchBatch, Workspace};

impl Workspace {
    /// Apply a text update and write it to disk.
    pub fn update_file(&self, path: &Path, content: String) -> Result<UpdateBatch, DaemonError> {
        self.write_file(path, FileChange::Text { content })
    }

    /// Apply a text update without writing to disk.
    pub fn update_memory_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<UpdateBatch, DaemonError> {
        self.workspace
            .apply_file(path, FileChange::Text { content })
            .map_err(DaemonError::from)
    }

    /// Mark a file as removed without touching the filesystem.
    pub fn remove_virtual_file(&self, path: &Path) -> Result<UpdateBatch, DaemonError> {
        self.workspace
            .apply_file(path, FileChange::Removed)
            .map_err(DaemonError::from)
    }

    /// Save text content from the editor or filesystem.
    pub fn save_text_file(
        &self,
        path: &Path,
        content: Option<String>,
    ) -> Result<UpdateBatch, DaemonError> {
        let content = match content {
            Some(content) => content,
            None => self
                .repository
                .file_system()
                .read_to_string(path)
                .map_err(|error| DaemonError::FileWrite {
                    path: path.to_path_buf(),
                    error,
                })?,
        };

        self.workspace
            .save_file(path, FileChange::Text { content })
            .map_err(DaemonError::from)
    }

    /// Save binary content from the editor or filesystem.
    pub fn save_bytes_file(
        &self,
        path: &Path,
        content: Option<Vec<u8>>,
    ) -> Result<UpdateBatch, DaemonError> {
        let content = match content {
            Some(content) => content,
            None => self.repository.file_system().read(path).map_err(|error| {
                DaemonError::FileWrite {
                    path: path.to_path_buf(),
                    error,
                }
            })?,
        };

        self.workspace
            .save_file(path, FileChange::Bytes { content })
            .map_err(DaemonError::from)
    }

    /// Write a file update to disk and workspace state.
    pub fn write_file(&self, path: &Path, update: FileChange) -> Result<UpdateBatch, DaemonError> {
        // write the update to disk first
        self.write_update_to_disk(path, &update)?;

        self.workspace
            .apply_file(path, update)
            .map_err(DaemonError::from)
    }

    /// Apply an atomic source update through the daemon.
    pub fn apply_source_update(
        &self,
        root: &Path,
        update: SourceUpdate,
    ) -> Result<SourceUpdateResult, DaemonError> {
        self.workspace
            .apply_source_update(root, update)
            .map_err(DaemonError::from)
    }

    /// Apply a watch event through the daemon.
    pub fn apply_watch_event(&self, event: &FileWatchEvent) -> UpdateBatch {
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
    pub fn apply_watch_batch(&self, batch: &WatchBatch) -> UpdateBatch {
        let mut result = UpdateBatch::default();

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
                result.updates.extend(workspace_result.updates);
                result.messages.extend(workspace_result.messages);
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
                    result.updates.extend(workspace_result.updates);
                    result.messages.extend(workspace_result.messages);
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
    pub fn reload_roots(&self, roots: &[PathBuf]) -> UpdateBatch {
        let result = self.workspace.reload_roots(roots);
        match result {
            Ok(result) => result,
            Err(error) => UpdateBatch {
                updates: Vec::new(),
                messages: vec![Message::warning(
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
                message: Some(Message::warning(
                    "watch_status_error",
                    format!("watch: {message}"),
                )),
            },
            FileWatchStatus::RescanRequested { reason, .. } => WatchStatusResult {
                should_reload: true,
                message: Some(Message {
                    kind: MessageKind::Info,
                    code: "watch_reload_requested".to_string(),
                    message: watch_reload_requested_message(reason).to_string(),
                }),
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
    message: Option<Message>,
}

/// Build a daemon message for workspace failures.
fn workspace_error_message(code: &str, error: &Error) -> Message {
    Message {
        kind: MessageKind::Error,
        code: code.to_string(),
        message: error.to_string(),
    }
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
