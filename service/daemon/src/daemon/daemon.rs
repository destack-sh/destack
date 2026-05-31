use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_service::{FileImage as ServiceFileImage, FileUpdateKind, LanguageService};
use destack_session::SessionEventHandler;
use destack_source::{Diagnostic, FileId, ModuleId};
use destack_workspace::{Ref, Repository};
use parking_lot::Mutex;

use super::DaemonMessage;
use crate::DaemonError;

/// Persistent process state for daemon clients.
#[derive(Clone)]
pub struct Daemon {
    /// The active repository for this daemon.
    pub repository: Arc<Repository>,
    /// Number of workers for each opened session.
    pub worker_limit: usize,
    /// Optional session event handler for in process daemon work.
    pub session_event_handler: Option<SessionEventHandler>,
    /// Shared language service.
    pub language_service: Arc<LanguageService>,
    /// Per-root handle lease counts.
    root_leases: Arc<Mutex<HashMap<PathBuf, usize>>>,
    /// Monotonic ids for private command refs.
    next_command_ref_id: Arc<AtomicU64>,
}

impl std::fmt::Debug for Daemon {
    /// Format the visible daemon state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Daemon")
            .field("repository", &self.repository)
            .field("worker_limit", &self.worker_limit)
            .field(
                "session_event_handler",
                &self.session_event_handler.is_some(),
            )
            .field("language_service", &self.language_service)
            .field("root_leases", &self.root_leases)
            .field("next_command_ref_id", &self.next_command_ref_id)
            .finish()
    }
}

impl Daemon {
    /// Create a daemon.
    pub fn new(
        repository: Arc<Repository>,
        worker_limit: usize,
        session_event_handler: Option<SessionEventHandler>,
    ) -> Self {
        let roots = vec![repository.workspace_root().to_path_buf()];

        Self::new_with_roots(repository, roots, worker_limit, session_event_handler)
    }

    /// Create a daemon with explicit language roots.
    pub fn new_with_roots(
        repository: Arc<Repository>,
        roots: Vec<PathBuf>,
        worker_limit: usize,
        session_event_handler: Option<SessionEventHandler>,
    ) -> Self {
        let language_service = LanguageService::new(
            repository.clone(),
            None,
            roots,
            worker_limit,
            session_event_handler.clone(),
        )
        .expect("language service initialization should not fail");

        Self {
            repository,
            worker_limit,
            session_event_handler,
            language_service: Arc::new(language_service),
            root_leases: Arc::new(Mutex::new(HashMap::new())),
            next_command_ref_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Allocate one private command ref for a root.
    pub(crate) fn next_command_ref(&self, root: &Path) -> Ref {
        let id = self.next_command_ref_id.fetch_add(1, Ordering::Relaxed);

        Ref::new(format!("command:{}:{id}", root.display()))
    }

    /// Return the number of tracked roots.
    #[cfg(test)]
    pub(crate) fn root_count(&self) -> usize {
        self.language_service.root_count()
    }

    /// Acquire a root lease.
    pub(crate) fn acquire_root(&self, root: &Path) -> Result<(), DaemonError> {
        self.language_service.open_root(root.to_path_buf())?;

        let mut leases = self.root_leases.lock();
        let lease_count = leases.entry(root.to_path_buf()).or_default();
        *lease_count += 1;

        Ok(())
    }

    /// Release a root lease and close when the last lease is dropped.
    pub(crate) fn release_root(&self, root: &Path) -> Result<bool, DaemonError> {
        let mut should_close = false;
        {
            let mut leases = self.root_leases.lock();
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
            self.language_service.close_root(root)?;
            return Ok(true);
        }

        Ok(false)
    }
}

/// Summary of a daemon update.
#[derive(Debug, Clone)]
pub struct DaemonUpdate {
    /// The module id that was updated.
    pub module_id: Option<ModuleId>,
    /// The file id for the updated module.
    pub file_id: FileId,
    /// File image when the updated file still exists.
    pub file: Option<ServiceFileImage>,
    /// The coarse change kind for this file.
    pub kind: FileUpdateKind,
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
