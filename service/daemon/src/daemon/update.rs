use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use destack_service::{
    FileMutation, FileUpdate, LanguageServiceError, LanguageServiceMessage,
    LanguageServiceMessageKind, LanguageServiceResult, ReloadReason,
};
use destack_source::{FileWatchEvent, FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};

use crate::{
    Daemon, DaemonError, DaemonMessage, DaemonMessageKind, DaemonReloadResult, DaemonUpdate,
    DaemonUpdateResult, DaemonWatchBatchResult, DaemonWatchEventResult, WatchBatch,
};

impl Daemon {
    /// Apply a text update and reanalyze the owning module.
    pub fn update_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileMutation::Text { content }, true)
    }

    /// Apply a text update without writing to the filesystem.
    pub fn update_virtual_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileMutation::Text { content }, false)
    }

    /// Mark a file as removed without touching the filesystem.
    pub fn remove_virtual_file(&self, path: &Path) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileMutation::Removed, false)
    }

    /// Apply a file update and optionally write to the filesystem.
    pub fn apply_file_update(
        &self,
        path: &Path,
        update: FileMutation,
        write_to_disk: bool,
    ) -> Result<DaemonUpdateResult, DaemonError> {
        // write the update to disk when requested
        if write_to_disk {
            self.write_update_to_disk(path, &update)?;
        }

        // apply the update through workspace service
        let workspace_result = self.workspace_service.apply_virtual_update(path, update)?;

        Ok(daemon_update_result_from_workspace(workspace_result))
    }

    /// Apply a watch event through the daemon.
    pub fn apply_watch_event(&self, event: &FileWatchEvent) -> DaemonWatchEventResult {
        // reuse the batch flow for single events
        let now = Instant::now();
        let batch = WatchBatch {
            events: vec![event.clone()],
            status: Vec::new(),
            started_at: now,
            ended_at: now,
            overflowed: matches!(event.kind, FileWatchEventKind::Overflow),
        };
        let result = self.apply_watch_batch(&batch);

        DaemonWatchEventResult {
            updates: result.updates,
            messages: result.messages,
        }
    }

    /// Apply a watch batch through the daemon.
    pub fn apply_watch_batch(&self, batch: &WatchBatch) -> DaemonWatchBatchResult {
        let mut result = DaemonWatchBatchResult::default();

        // apply watch statuses first
        let mut status_reload_reason = None;
        for status in &batch.status {
            let status_result = self.handle_watch_status(status);
            if status_reload_reason.is_none() {
                status_reload_reason = status_result.reload_reason;
            }
            if let Some(message) = status_result.message {
                result.messages.push(message);
            }
        }

        // apply file events through workspace service
        let has_overflow_event = batch
            .events
            .iter()
            .any(|event| matches!(event.kind, FileWatchEventKind::Overflow));
        match self
            .workspace_service
            .apply_watch_events(batch.events.clone(), Default::default())
        {
            Ok(workspace_result) => {
                let daemon_result = daemon_update_result_from_workspace(workspace_result);
                result.updates.extend(daemon_result.updates);
                result.messages.extend(daemon_result.messages);
            }
            Err(error) => {
                result.messages.push(workspace_service_error_message(
                    "watch_apply_failed",
                    &error,
                ));
            }
        }

        // handle overflow status batches without explicit overflow events
        if batch.overflowed && !has_overflow_event && status_reload_reason.is_none() {
            status_reload_reason = Some(ReloadReason::Overflow);
        }

        // reload eagerly when status requests a full refresh
        if let Some(reload_reason) = status_reload_reason {
            match self.workspace_service.reload_all_workspaces(reload_reason) {
                Ok(workspace_result) => {
                    let daemon_result = daemon_update_result_from_workspace(workspace_result);
                    result.updates.extend(daemon_result.updates);
                    result.messages.extend(daemon_result.messages);
                }
                Err(error) => {
                    result.messages.push(workspace_service_error_message(
                        "watch_reload_failed",
                        &error,
                    ));
                }
            }
        }

        result
    }

    /// Reload tracked filesystem state for the provided roots.
    pub fn reload_workspaces(&self, roots: &[PathBuf]) -> DaemonReloadResult {
        let result = self.workspace_service.reload_workspaces(roots);
        match result {
            Ok(result) => {
                let daemon_result = daemon_update_result_from_workspace(result);
                DaemonReloadResult {
                    updates: daemon_result.updates,
                    messages: daemon_result.messages,
                }
            }
            Err(error) => DaemonReloadResult {
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
    fn write_update_to_disk(&self, path: &Path, update: &FileMutation) -> Result<(), DaemonError> {
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
            FileMutation::Text { content } => {
                self.repository
                    .file_system()
                    .write_string(path, content)
                    .map_err(|error| DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    })?;
            }
            FileMutation::Bytes { content } => {
                self.repository
                    .file_system()
                    .write(path, content)
                    .map_err(|error| DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    })?;
            }
            FileMutation::Removed => {
                if let Err(error) = self.repository.file_system().remove_file(path)
                    && error.kind() != io::ErrorKind::NotFound
                {
                    return Err(DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    });
                }
            }
        }

        Ok(())
    }

    /// Handle watch status events.
    fn handle_watch_status(&self, status: &FileWatchStatus) -> WatchStatusResult {
        match status {
            FileWatchStatus::Error { message } => WatchStatusResult {
                reload_reason: None,
                message: Some(DaemonMessage::new(
                    DaemonMessageKind::Warning,
                    "watch_status_error",
                    format!("watch: {message}"),
                )),
            },
            FileWatchStatus::RescanRequested { reason, .. } => WatchStatusResult {
                reload_reason: Some(reload_reason_from_watch_reason(reason)),
                message: Some(DaemonMessage::new(
                    DaemonMessageKind::Info,
                    "watch_reload_requested",
                    watch_reload_requested_message(reason),
                )),
            },
            FileWatchStatus::Ready { .. } => WatchStatusResult {
                reload_reason: None,
                message: None,
            },
            FileWatchStatus::Stopped => WatchStatusResult {
                reload_reason: None,
                message: None,
            },
        }
    }
}

/// Status handling result.
#[derive(Debug, Clone, Default)]
struct WatchStatusResult {
    /// Optional reload reason.
    reload_reason: Option<ReloadReason>,
    /// Optional surfaced message.
    message: Option<DaemonMessage>,
}

/// Convert a watch rescan reason to workspace service reason.
fn reload_reason_from_watch_reason(reason: &FileWatchRescanReason) -> ReloadReason {
    match reason {
        FileWatchRescanReason::Startup => ReloadReason::Startup,
        FileWatchRescanReason::Overflow => ReloadReason::Overflow,
        FileWatchRescanReason::Manual => ReloadReason::Manual,
        FileWatchRescanReason::Update => ReloadReason::Update,
    }
}

/// Convert a workspace update result into daemon shape.
fn daemon_update_result_from_workspace(result: LanguageServiceResult) -> DaemonUpdateResult {
    DaemonUpdateResult {
        updates: result
            .updates
            .into_iter()
            .map(daemon_update_from_workspace)
            .collect(),
        messages: daemon_messages_from_workspace(result.messages),
    }
}

/// Convert workspace service messages to daemon messages.
fn daemon_messages_from_workspace(messages: Vec<LanguageServiceMessage>) -> Vec<DaemonMessage> {
    messages
        .into_iter()
        .map(|message| {
            let kind = match message.kind {
                LanguageServiceMessageKind::Info => DaemonMessageKind::Info,
                LanguageServiceMessageKind::Warning => DaemonMessageKind::Warning,
                LanguageServiceMessageKind::Error => DaemonMessageKind::Error,
            };

            DaemonMessage::new(kind, message.code, message.message)
        })
        .collect()
}

/// Build a daemon message for workspace service failures.
fn workspace_service_error_message(code: &str, error: &LanguageServiceError) -> DaemonMessage {
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
