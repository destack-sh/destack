use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::{Compiler, CompilerOptions};
use destack_service::{FileChangeKind, FileSnapshot as WorkspaceFileSnapshot, LanguageService};
use destack_session::{SessionEventHandler, SessionObservationHandler};
use destack_source::{Diagnostic, FileId, ModuleId};
use destack_workspace::Repository;
use parking_lot::Mutex;

use super::DaemonMessage;
use crate::DaemonError;

/// Persistent daemon state for toolchain services.
/// Basically, we wrap LanguageServices in a stateful central place.
#[derive(Clone)]
pub struct Daemon {
    /// The active repository for this daemon.
    pub repository: Arc<Repository>,
    /// Default compiler options for daemon work.
    pub compiler_options: CompilerOptions,
    /// Optional session event handler for in process daemon work.
    pub session_event_handler: Option<SessionEventHandler>,
    /// Optional session observation handler for in process daemon work.
    pub session_observation_handler: Option<SessionObservationHandler>,
    /// Shared workspace orchestration service.
    pub workspace_service: Arc<LanguageService>,
    /// Per-root workspace handle lease counts.
    workspace_leases: Arc<Mutex<HashMap<PathBuf, usize>>>,
}

impl std::fmt::Debug for Daemon {
    /// Format the visible daemon state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Daemon")
            .field("repository", &self.repository)
            .field("compiler_options", &self.compiler_options)
            .field(
                "session_event_handler",
                &self.session_event_handler.is_some(),
            )
            .field(
                "session_observation_handler",
                &self.session_observation_handler.is_some(),
            )
            .field("workspace_service", &self.workspace_service)
            .field("workspace_leases", &self.workspace_leases)
            .finish()
    }
}

impl Daemon {
    /// Create a daemon for the given repository.
    pub fn new(repository: Arc<Repository>) -> Self {
        Self::with_options(repository, CompilerOptions::default(), None, None)
    }

    /// Create a daemon with explicit compiler options.
    pub fn with_options(
        repository: Arc<Repository>,
        compiler_options: CompilerOptions,
        session_event_handler: Option<SessionEventHandler>,
        session_observation_handler: Option<SessionObservationHandler>,
    ) -> Self {
        let roots = vec![repository.workspace_root().to_path_buf()];
        let workspace_service = LanguageService::with_options(
            repository.clone(),
            None,
            roots,
            compiler_options.clone(),
            session_event_handler.clone(),
            session_observation_handler.clone(),
        )
        .expect("workspace service initialization should not fail");

        Self {
            repository,
            compiler_options,
            session_event_handler,
            session_observation_handler,
            workspace_service: Arc::new(workspace_service),
            workspace_leases: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Return the number of tracked workspace roots.
    #[cfg(test)]
    pub(crate) fn workspace_root_count(&self) -> usize {
        self.workspace_service.workspace_root_count()
    }

    /// Acquire a workspace root lease.
    pub(crate) fn acquire_workspace_root(&self, root: &Path) -> Result<(), DaemonError> {
        self.workspace_service
            .open_workspace_root(root.to_path_buf())?;

        let mut leases = self.workspace_leases.lock();
        let lease_count = leases.entry(root.to_path_buf()).or_default();
        *lease_count += 1;

        Ok(())
    }

    /// Release a workspace root lease and close when the last lease is dropped.
    pub(crate) fn release_workspace_root(&self, root: &Path) -> Result<bool, DaemonError> {
        let mut should_close = false;
        {
            let mut leases = self.workspace_leases.lock();
            if let Some(lease_count) = leases.get_mut(root) {
                if *lease_count > 1 {
                    *lease_count -= 1;
                } else {
                    leases.remove(root);
                    should_close = true;
                }
            } else {
                return Ok(false);
            }
        }

        if should_close {
            self.workspace_service.close_workspace_root(root)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Return the compiler for a workspace root.
    pub(crate) fn compiler_for_workspace_root(
        &self,
        root: &Path,
    ) -> Result<Arc<Compiler>, DaemonError> {
        self.workspace_service
            .compiler_for_workspace_root(root)
            .map_err(Into::into)
    }
}

/// Summary of a daemon update.
#[derive(Debug, Clone)]
pub struct DaemonUpdate {
    /// The module id that was updated.
    pub module_id: Option<ModuleId>,
    /// The file id for the updated module.
    pub file_id: FileId,
    /// File snapshot for the updated file.
    pub file: WorkspaceFileSnapshot,
    /// The coarse change kind for this file.
    pub kind: FileChangeKind,
    /// Diagnostics for the updated file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Result of applying an update through the daemon.
#[derive(Debug, Clone, Default)]
pub struct DaemonUpdateResult {
    /// Updates produced by the operation.
    pub updates: Vec<DaemonUpdate>,
    /// Warnings or errors to surface to the caller.
    pub messages: Vec<DaemonMessage>,
}

impl DaemonUpdateResult {
    /// Return true when the update operation produced updates.
    pub fn updated(&self) -> bool {
        !self.updates.is_empty()
    }
}

/// Summary of a watch event applied through the daemon.
#[derive(Debug, Clone, Default)]
pub struct DaemonWatchEventResult {
    /// Updates produced by this event.
    pub updates: Vec<DaemonUpdate>,
    /// Warnings or errors to surface to the caller.
    pub messages: Vec<DaemonMessage>,
}

impl DaemonWatchEventResult {
    /// Return true when the event produced updates.
    pub fn updated(&self) -> bool {
        !self.updates.is_empty()
    }
}

/// Summary of a watch batch applied through the daemon.
#[derive(Debug, Clone, Default)]
pub struct DaemonWatchBatchResult {
    /// Updates produced by this batch.
    pub updates: Vec<DaemonUpdate>,
    /// Warnings or errors to surface to the caller.
    pub messages: Vec<DaemonMessage>,
}

impl DaemonWatchBatchResult {
    /// Return true when the batch produced updates.
    pub fn updated(&self) -> bool {
        !self.updates.is_empty()
    }
}

/// Summary of a filesystem reload applied through the daemon.
#[derive(Debug, Clone, Default)]
pub struct DaemonReloadResult {
    /// Updates produced by the reload.
    pub updates: Vec<DaemonUpdate>,
    /// Warnings or errors to surface to the caller.
    pub messages: Vec<DaemonMessage>,
}

impl DaemonReloadResult {
    /// Return true when the reload produced updates.
    pub fn updated(&self) -> bool {
        !self.updates.is_empty()
    }
}
