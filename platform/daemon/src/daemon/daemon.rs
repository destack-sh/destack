use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::{Compiler, CompilerOptions};
use destack_source::{Diagnostic, FileId, ModuleId};
use destack_workspace::{InvalidationPlan, Session};
use destack_workspace_service::{
    AnalyzeOutcome, FileSnapshot as WorkspaceFileSnapshot, WorkspaceHandleId, WorkspaceService,
};
use parking_lot::Mutex;

use super::DaemonMessage;
use crate::DaemonError;

/// Persistent daemon state for toolchain services.
/// Basically, we wrap WorkspaceServices in a stateful central place.
#[derive(Debug, Clone)]
pub struct Daemon {
    /// The active session for this daemon.
    pub session: Arc<Session>,
    /// Default compiler options for daemon work.
    pub compiler_options: CompilerOptions,
    /// Shared workspace orchestration service.
    pub workspace_service: Arc<WorkspaceService>,
    /// Per-root workspace handle lease counts.
    workspace_leases: Arc<Mutex<HashMap<PathBuf, usize>>>,
}

impl Daemon {
    /// Create a daemon for the given session.
    pub fn new(session: Arc<Session>) -> Self {
        Self::with_options(session, CompilerOptions::default())
    }

    /// Create a daemon with explicit compiler options.
    pub fn with_options(session: Arc<Session>, compiler_options: CompilerOptions) -> Self {
        let mut roots: Vec<PathBuf> = session
            .programs
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        if roots.is_empty() {
            roots.push(session.workspace_root());
        }
        let workspace_service =
            WorkspaceService::with_options(session.clone(), roots, compiler_options.clone())
                .expect("workspace service initialization should not fail");

        Self {
            session,
            compiler_options,
            workspace_service: Arc::new(workspace_service),
            workspace_leases: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Return the number of tracked program handles.
    #[cfg(test)]
    pub(crate) fn program_handle_count(&self) -> usize {
        self.workspace_service.program_handle_count()
    }

    /// Acquire a workspace root lease and return its stable handle.
    pub(crate) fn acquire_workspace_root(
        &self,
        root: &Path,
    ) -> Result<WorkspaceHandleId, DaemonError> {
        self.workspace_service
            .open_workspace_root(root.to_path_buf())?;
        let handle = self.workspace_service.handle_for_root(root)?;

        let mut leases = self.workspace_leases.lock();
        let lease_count = leases.entry(root.to_path_buf()).or_default();
        *lease_count += 1;

        Ok(handle)
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
    pub(crate) fn compiler_for_root(&self, root: &Path) -> Arc<Compiler> {
        self.workspace_service.compiler_for_root(root)
    }

    /// Ensure a module for the given path is analyzed.
    pub fn analyze_path(&self, path: &Path) -> Result<AnalyzeOutcome, DaemonError> {
        self.workspace_service
            .analyze_path(path)
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
    /// The invalidation summary for the update.
    pub invalidation: InvalidationPlan,
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

/// Summary of a rescan applied through the daemon.
#[derive(Debug, Clone, Default)]
pub struct DaemonRescanResult {
    /// Updates produced by the rescan.
    pub updates: Vec<DaemonUpdate>,
    /// Warnings or errors to surface to the caller.
    pub messages: Vec<DaemonMessage>,
}

impl DaemonRescanResult {
    /// Return true when the rescan produced updates.
    pub fn updated(&self) -> bool {
        !self.updates.is_empty()
    }
}
