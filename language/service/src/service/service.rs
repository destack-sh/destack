use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use dashmap::DashMap;
use destack_compiler::CompilerOptions;
use destack_workspace::Session;

use super::workspace::WorkspaceHandle;
use super::{LanguageServiceError, WorkspaceHandleId};

/// Local workspace backed service used by tooling integrations.
#[derive(Debug)]
pub struct LanguageService {
    /// Session for workspace resolution.
    pub(super) session: Arc<Session>,
    /// Compiler execution options for local analysis work.
    pub(super) compiler_execution_options: CompilerOptions,
    /// Workspace handles keyed by stable handle id.
    pub(super) handles_by_id: DashMap<WorkspaceHandleId, Arc<WorkspaceHandle>>,
    /// Workspace handle ids keyed by root path.
    pub(super) handle_ids_by_root: DashMap<PathBuf, WorkspaceHandleId>,
    /// Next handle id.
    pub(super) next_handle_id: AtomicU64,
}

impl LanguageService {
    /// Create a local workspace service for the provided roots.
    pub fn new(session: Arc<Session>, roots: Vec<PathBuf>) -> Result<Self, LanguageServiceError> {
        Self::with_options(session, roots, CompilerOptions::default())
    }
}
