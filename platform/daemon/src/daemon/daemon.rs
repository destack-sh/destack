use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_compiler::CompilerOptions;
use destack_source::{Diagnostic, FileId, ModuleId};
use destack_workspace::{InvalidationPlan, Session};

use super::DaemonMessage;
use super::program::ProgramHandle;
use crate::protocol::FileSnapshot;

/// Persistent daemon state for toolchain services.
#[derive(Debug, Clone)]
pub struct Daemon {
    /// The active session for this daemon.
    pub session: Arc<Session>,
    /// Default compiler options for daemon work.
    pub compiler_options: CompilerOptions,
    /// Per program daemon handle.
    pub(super) program_handles: Arc<DashMap<PathBuf, Arc<ProgramHandle>>>,
}

impl Daemon {
    /// Create a daemon for the given session.
    pub fn new(session: Arc<Session>) -> Self {
        Self {
            session,
            compiler_options: CompilerOptions::default(),
            program_handles: Arc::new(DashMap::new()),
        }
    }

    /// Create a daemon with explicit compiler options.
    pub fn with_options(session: Arc<Session>, compiler_options: CompilerOptions) -> Self {
        Self {
            session,
            compiler_options,
            program_handles: Arc::new(DashMap::new()),
        }
    }

    /// Return the number of tracked program handles.
    #[cfg(test)]
    pub(crate) fn program_handle_count(&self) -> usize {
        self.program_handles.len()
    }

    /// Drop the program handle for a workspace root.
    pub fn remove_program_handle(&self, root: &Path) -> bool {
        self.program_handles.remove(root).is_some()
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
    pub file: FileSnapshot,
    /// The invalidation summary for the update.
    pub invalidation: InvalidationPlan,
    /// Diagnostics for the updated file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Summary of a watch event applied through the daemon.
#[derive(Debug, Clone, Default)]
pub struct DaemonWatchEventResult {
    /// Updates produced by this event.
    pub updates: Vec<DaemonUpdate>,
    /// Whether a rescan is required.
    pub rescan: bool,
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
    /// Whether a rescan is required.
    pub rescan: bool,
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
