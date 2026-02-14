use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use dashmap::DashMap;
use destack_compiler::CompilerOptions;
use destack_workspace::Session;

use super::workspace::ProgramHandle;
use super::{LanguageServiceError, WorkspaceHandleId};

/// Local workspace backed service used by tooling integrations.
#[derive(Debug)]
pub struct LanguageService {
    /// Session for workspace resolution.
    pub(super) session: Arc<Session>,
    /// Compiler options for local analysis.
    pub(super) compiler_options: CompilerOptions,
    /// Program handles keyed by root path.
    pub(super) program_handles: DashMap<PathBuf, Arc<ProgramHandle>>,
    /// Handle ids keyed by root path.
    pub(super) handles_by_root: DashMap<PathBuf, WorkspaceHandleId>,
    /// Root paths keyed by handle id.
    pub(super) roots_by_handle: DashMap<WorkspaceHandleId, PathBuf>,
    /// Next handle id.
    pub(super) next_handle_id: AtomicU64,
}

impl LanguageService {
    /// Create a local workspace service for the provided roots.
    pub fn new(session: Arc<Session>, roots: Vec<PathBuf>) -> Result<Self, LanguageServiceError> {
        Self::with_options(session, roots, CompilerOptions::default())
    }
}
