use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_compiler::CompilerOptions;
use destack_source::{OverlayFileSystem, Uri};
use destack_workspace::Repository;

use super::LanguageServiceError;
use super::workspace::WorkspaceSession;

/// Local workspace backed service used by tooling integrations.
#[derive(Debug)]
pub struct LanguageService {
    /// Repository for workspace resolution.
    pub(super) repository: Arc<Repository>,
    /// Overlay filesystem for tracked document reads when available.
    pub(super) overlay_fs: Option<Arc<OverlayFileSystem>>,
    /// Compiler execution options for local analysis work.
    pub(super) compiler_execution_options: CompilerOptions,
    /// Workspace state keyed by root path.
    pub(super) workspaces_by_root: DashMap<PathBuf, Arc<WorkspaceSession>>,
}

impl LanguageService {
    /// Create a local workspace service for the provided roots.
    pub fn new(
        repository: Arc<Repository>,
        roots: Vec<PathBuf>,
    ) -> Result<Self, LanguageServiceError> {
        Self::with_options(repository, None, roots, CompilerOptions::default())
    }

    /// Return true when a path is tracked as one open document.
    pub fn has_tracked_document_for_path(&self, path: &Path) -> bool {
        let Some(session) = self.tracked_workspace_for_path(path) else {
            return false;
        };

        session.has_tracked_document_for_path(path)
    }

    /// Return one tracked document snapshot for a path.
    pub fn tracked_document_for_path(&self, path: &Path) -> Option<(Uri, i32, String)> {
        let session = self.tracked_workspace_for_path(path)?;
        session.tracked_document_for_path(path)
    }

    /// Return the tracked open documents keyed by path.
    pub fn tracked_documents(&self) -> Vec<(PathBuf, Uri, i32)> {
        let mut documents = Vec::new();
        for session in self.workspaces_by_root.iter() {
            documents.extend(session.value().tracked_document_identities());
        }

        documents
    }
}
