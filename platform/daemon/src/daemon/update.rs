use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use destack_source::{FileWatchEvent, FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};
use destack_workspace::FileUpdate;
use destack_workspace_service::{
    RescanReason, WorkspaceMessage, WorkspaceMessageKind, WorkspaceServiceError,
    WorkspaceServiceResult, WorkspaceUpdateRecord,
};

use crate::{
    Daemon, DaemonError, DaemonMessage, DaemonMessageKind, DaemonRescanResult, DaemonUpdate,
    DaemonUpdateResult, DaemonWatchBatchResult, DaemonWatchEventResult, WatchBatch,
};

impl Daemon {
    /// Apply a text update and reanalyze the owning module.
    pub fn update_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileUpdate::Text { content }, true)
    }

    /// Apply a text update without writing to the filesystem.
    pub fn update_virtual_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileUpdate::Text { content }, false)
    }

    /// Mark a file as removed without touching the filesystem.
    pub fn remove_virtual_file(&self, path: &Path) -> Result<DaemonUpdateResult, DaemonError> {
        self.apply_file_update(path, FileUpdate::Removed, false)
    }

    /// Apply a file update and optionally write to the filesystem.
    pub fn apply_file_update(
        &self,
        path: &Path,
        update: FileUpdate,
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
        let mut status_rescan_reason = None;
        for status in &batch.status {
            let status_result = self.handle_watch_status(status);
            if status_rescan_reason.is_none() {
                status_rescan_reason = status_result.rescan_reason;
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
            .apply_watch_events(batch.events.clone())
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
        if batch.overflowed && !has_overflow_event && status_rescan_reason.is_none() {
            status_rescan_reason = Some(RescanReason::Overflow);
        }

        // rescan eagerly when status requests a full refresh
        if let Some(rescan_reason) = status_rescan_reason {
            match self.workspace_service.rescan_all(rescan_reason) {
                Ok(workspace_result) => {
                    let daemon_result = daemon_update_result_from_workspace(workspace_result);
                    result.updates.extend(daemon_result.updates);
                    result.messages.extend(daemon_result.messages);
                }
                Err(error) => {
                    result.messages.push(workspace_service_error_message(
                        "watch_rescan_failed",
                        &error,
                    ));
                }
            }
        }

        result
    }

    /// Rescan tracked files for the provided roots.
    pub fn rescan_roots(&self, roots: &[PathBuf]) -> DaemonRescanResult {
        self.rescan_roots_with_mode(roots, false)
    }

    /// Rescan tracked files and analyze updated modules.
    pub fn rescan_roots_with_analysis(&self, roots: &[PathBuf]) -> DaemonRescanResult {
        self.rescan_roots_with_mode(roots, true)
    }

    /// Rescan tracked files for the provided roots and analyze when requested.
    fn rescan_roots_with_mode(&self, roots: &[PathBuf], analyze: bool) -> DaemonRescanResult {
        let result = self.workspace_service.rescan_roots(roots, analyze);
        match result {
            Ok(result) => {
                let daemon_result = daemon_update_result_from_workspace(result);
                DaemonRescanResult {
                    updates: daemon_result.updates,
                    messages: daemon_result.messages,
                }
            }
            Err(error) => DaemonRescanResult {
                updates: Vec::new(),
                messages: vec![DaemonMessage::new(
                    DaemonMessageKind::Warning,
                    "rescan_analyze_failed",
                    format!("watch: failed to analyze updated modules: {error}"),
                )],
            },
        }
    }

    /// Write a file update to disk before applying it.
    fn write_update_to_disk(&self, path: &Path, update: &FileUpdate) -> Result<(), DaemonError> {
        let parent = path.parent();
        if let Some(parent) = parent {
            self.session
                .fs
                .create_dir_all(parent)
                .map_err(|error| DaemonError::FileWrite {
                    path: parent.to_path_buf(),
                    error,
                })?;
        }

        match update {
            FileUpdate::Text { content } => {
                self.session
                    .fs
                    .write_string(path, content)
                    .map_err(|error| DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    })?;
            }
            FileUpdate::Bytes { content } => {
                self.session
                    .fs
                    .write(path, content)
                    .map_err(|error| DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    })?;
            }
            FileUpdate::Removed => {
                if let Err(error) = self.session.fs.remove_file(path)
                    && error.kind() != io::ErrorKind::NotFound
                {
                    return Err(DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    });
                }
            }
            FileUpdate::Touch => {}
        }

        Ok(())
    }

    /// Handle watch status events.
    fn handle_watch_status(&self, status: &FileWatchStatus) -> WatchStatusResult {
        match status {
            FileWatchStatus::Error { message } => WatchStatusResult {
                rescan_reason: None,
                message: Some(DaemonMessage::new(
                    DaemonMessageKind::Warning,
                    "watch_status_error",
                    format!("watch: {message}"),
                )),
            },
            FileWatchStatus::RescanRequested { reason, .. } => WatchStatusResult {
                rescan_reason: Some(rescan_reason_from_watch_reason(reason)),
                message: Some(DaemonMessage::new(
                    DaemonMessageKind::Info,
                    "watch_rescan_requested",
                    watch_rescan_requested_message(reason),
                )),
            },
            FileWatchStatus::Ready { .. } => WatchStatusResult {
                rescan_reason: None,
                message: None,
            },
            FileWatchStatus::Stopped => WatchStatusResult {
                rescan_reason: None,
                message: None,
            },
        }
    }
}

/// Status handling result.
#[derive(Debug, Clone, Default)]
struct WatchStatusResult {
    /// Optional rescan reason.
    rescan_reason: Option<RescanReason>,
    /// Optional surfaced message.
    message: Option<DaemonMessage>,
}

/// Convert a watch rescan reason to workspace service reason.
fn rescan_reason_from_watch_reason(reason: &FileWatchRescanReason) -> RescanReason {
    match reason {
        FileWatchRescanReason::Startup => RescanReason::Startup,
        FileWatchRescanReason::Overflow => RescanReason::Overflow,
        FileWatchRescanReason::Manual => RescanReason::Manual,
        FileWatchRescanReason::Update => RescanReason::Update,
    }
}

/// Convert a workspace update result into daemon shape.
fn daemon_update_result_from_workspace(result: WorkspaceServiceResult) -> DaemonUpdateResult {
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
fn daemon_messages_from_workspace(messages: Vec<WorkspaceMessage>) -> Vec<DaemonMessage> {
    messages
        .into_iter()
        .map(|message| {
            let kind = match message.kind {
                WorkspaceMessageKind::Info => DaemonMessageKind::Info,
                WorkspaceMessageKind::Warning => DaemonMessageKind::Warning,
                WorkspaceMessageKind::Error => DaemonMessageKind::Error,
            };

            DaemonMessage::new(kind, message.code, message.message)
        })
        .collect()
}

/// Build a daemon message for workspace service failures.
fn workspace_service_error_message(code: &str, error: &WorkspaceServiceError) -> DaemonMessage {
    DaemonMessage::new(DaemonMessageKind::Error, code, error.to_string())
}

/// Build a watch rescan requested message.
fn watch_rescan_requested_message(reason: &FileWatchRescanReason) -> &'static str {
    match reason {
        FileWatchRescanReason::Startup => "watch: rescan requested at startup",
        FileWatchRescanReason::Overflow => "watch: rescan requested after overflow",
        FileWatchRescanReason::Manual => "watch: rescan requested",
        FileWatchRescanReason::Update => "watch: rescan requested after update",
    }
}

/// Convert a workspace update into daemon shape.
fn daemon_update_from_workspace(update: WorkspaceUpdateRecord) -> DaemonUpdate {
    DaemonUpdate {
        module_id: update.module_id,
        file_id: update.file_id,
        file: update.file,
        invalidation: update.invalidation,
        diagnostics: update.diagnostics,
    }
}
