use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_compiler::CompilerOptions;
use destack_session::{Session, SessionEventHandler};
use destack_source::{OverlayFileSystem, Uri};
use destack_workspace::Repository;

use super::LanguageServiceError;

/// Local workspace backed service used by tooling integrations.
pub struct LanguageService {
    /// Repository for workspace resolution.
    pub(super) repository: Arc<Repository>,
    /// Overlay filesystem for tracked document reads when available.
    pub(super) overlay_fs: Option<Arc<OverlayFileSystem>>,
    /// Compiler execution options for local analysis work.
    pub(super) compiler_execution_options: CompilerOptions,
    /// Optional session event handler for local progress reporting.
    pub(super) session_event_handler: Option<SessionEventHandler>,
    /// Workspace state keyed by root path.
    pub(super) workspaces_by_root: DashMap<PathBuf, Arc<Session>>,
}

impl std::fmt::Debug for LanguageService {
    /// Format the visible language service state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LanguageService")
            .field("repository", &self.repository)
            .field("overlay_fs", &self.overlay_fs)
            .field(
                "compiler_execution_options",
                &self.compiler_execution_options,
            )
            .field(
                "session_event_handler",
                &self.session_event_handler.is_some(),
            )
            .field("workspaces_by_root", &self.workspaces_by_root)
            .finish()
    }
}

impl LanguageService {
    /// Create a local workspace service for the provided roots.
    pub fn new(
        repository: Arc<Repository>,
        roots: Vec<PathBuf>,
    ) -> Result<Self, LanguageServiceError> {
        Self::with_options(repository, None, roots, CompilerOptions::default(), None)
    }

    /// Return true when a path is tracked as one open document.
    pub fn has_tracked_document_for_path(&self, path: &Path) -> bool {
        let Some(session) = self.tracked_workspace_for_path(path) else {
            return false;
        };

        session.has_open_file_for_path(path)
    }

    /// Return one tracked document snapshot for a path.
    pub fn tracked_document_for_path(
        &self,
        path: &Path,
    ) -> Result<Option<(Uri, i32, String)>, LanguageServiceError> {
        let Some(session) = self.tracked_workspace_for_path(path) else {
            return Ok(None);
        };

        session
            .open_file_for_path(path)
            .map_err(|error| LanguageServiceError::Internal {
                detail: format!(
                    "failed to read tracked document {}: {error}",
                    path.display(),
                ),
            })
    }

    /// Return the tracked open documents keyed by path.
    pub fn tracked_documents(&self) -> Vec<(PathBuf, Uri, i32)> {
        let mut documents = Vec::new();
        for session in self.workspaces_by_root.iter() {
            documents.extend(session.value().open_file_identities());
        }

        documents
    }
}
