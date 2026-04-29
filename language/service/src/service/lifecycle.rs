use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::{Compiler, CompilerOptions};
use destack_linter::Linter;
use destack_session::{Session, SessionEventHandler, canonical_path_or_original};
use destack_source::OverlayFileSystem;
use destack_workspace::{Ref, Repository, Revision};

use super::{LanguageService, LanguageServiceError};

impl LanguageService {
    /// Resolve the owning workspace root for any workspace-scoped path.
    fn workspace_root_for_owned_path(&self, path: &Path) -> Option<PathBuf> {
        let canonical_path = canonical_path_or_original(path);
        let mut best_root = None;
        let mut best_depth = 0usize;

        for entry in self.workspaces_by_root.iter() {
            let root = entry.key();
            let canonical_root = canonical_path_or_original(root);

            let matches_root =
                path.starts_with(root) || canonical_path.starts_with(&canonical_root);
            if !matches_root {
                continue;
            }

            let depth = canonical_root.components().count();
            if depth <= best_depth {
                continue;
            }

            best_depth = depth;
            best_root = Some(root.clone());
        }

        best_root
    }

    /// Resolve the owning workspace root for any workspace-owned path.
    pub fn workspace_root_for_path(&self, path: &Path) -> Result<PathBuf, LanguageServiceError> {
        self.workspace_root_for_owned_path(path).ok_or_else(|| {
            LanguageServiceError::PathNotInWorkspace {
                path: path.to_path_buf(),
            }
        })
    }

    /// Resolve the owning workspace root for a semantic path.
    fn semantic_workspace_root_for_path(
        &self,
        path: &Path,
    ) -> Result<Option<PathBuf>, LanguageServiceError> {
        let canonical_path = canonical_path_or_original(path);
        let mut best_root = None;
        let mut best_depth = 0usize;

        for workspace in self.workspaces_by_root.iter() {
            let is_member = workspace.value().owns_semantic_path(path)?
                || (canonical_path != path
                    && workspace.value().owns_semantic_path(&canonical_path)?);
            if !is_member {
                continue;
            }

            let canonical_root = canonical_path_or_original(workspace.value().root());
            let depth = canonical_root.components().count();
            if depth <= best_depth {
                continue;
            }

            best_depth = depth;
            best_root = Some(workspace.value().root().to_path_buf());
        }

        Ok(best_root)
    }

    /// Resolve the workspace that already owns one tracked document.
    pub(super) fn tracked_workspace_for_path(&self, path: &Path) -> Option<Arc<Session>> {
        let canonical_path = canonical_path_or_original(path);

        for workspace in self.workspaces_by_root.iter() {
            if workspace.value().has_open_file_for_path(&canonical_path) {
                return Some(Arc::clone(workspace.value()));
            }
        }

        None
    }

    /// Resolve the workspace that should own one tracked document path.
    pub(super) fn workspace_for_document_path(
        &self,
        path: &Path,
    ) -> Result<Arc<Session>, LanguageServiceError> {
        // prefer the existing tracked owner before rediscovering semantic routing
        if let Some(workspace) = self.tracked_workspace_for_path(path) {
            return Ok(workspace);
        }

        // then use workspace root ownership for config and newly created files
        if let Some(root) = self.workspace_root_for_owned_path(path) {
            return self.workspace_for_root(&root);
        }

        self.workspace_for_path(path)
    }

    /// Create a local workspace service with explicit compiler options.
    pub fn with_options(
        repository: Arc<Repository>,
        overlay_fs: Option<Arc<OverlayFileSystem>>,
        roots: Vec<PathBuf>,
        compiler_options: CompilerOptions,
        session_event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, LanguageServiceError> {
        let service = Self {
            repository,
            overlay_fs,
            compiler_execution_options: compiler_options,
            session_event_handler,
            workspaces_by_root: dashmap::DashMap::new(),
        };

        for root in roots {
            service.open_workspace_root(root)?;
        }

        Ok(service)
    }

    /// Ensure the workspace root is opened.
    pub fn open_workspace_root(&self, root: PathBuf) -> Result<(), LanguageServiceError> {
        if self.workspaces_by_root.contains_key(&root) {
            return Ok(());
        }

        let session = Arc::new(self.build_session(root.clone())?);
        self.workspaces_by_root
            .insert(root.clone(), Arc::clone(&session));

        // synchronize the current file system state for the new workspace ref
        let result = session.discover_filesystem();
        let Err(error) = result else {
            return Ok(());
        };

        // rollback a failed root open so service state stays consistent
        self.workspaces_by_root.remove(root.as_path());

        Err(LanguageServiceError::from(error))
    }

    /// Close an opened workspace root.
    pub fn close_workspace_root(&self, root: &Path) -> Result<(), LanguageServiceError> {
        let root = root.to_path_buf();
        let _workspace = self.workspaces_by_root.remove(root.as_path());

        Ok(())
    }

    /// Return true when a workspace root is active.
    pub fn has_workspace_root(&self, root: &Path) -> bool {
        let root = root.to_path_buf();
        self.workspaces_by_root.contains_key(root.as_path())
    }

    /// Remove an opened workspace root and report whether it existed.
    pub fn remove_workspace_root(&self, root: &Path) -> Result<bool, LanguageServiceError> {
        let existed = self.has_workspace_root(root);
        self.close_workspace_root(root)?;

        Ok(existed)
    }

    /// Clear all cache entries for every workspace root.
    pub fn clear_cache_all(&self) -> Result<(), LanguageServiceError> {
        let cache_dir = self.repository.cache_directory();
        if !cache_dir.exists() {
            return Ok(());
        }

        std::fs::remove_dir_all(&cache_dir).map_err(|error| {
            LanguageServiceError::CacheClearFailed {
                path: cache_dir.clone(),
                detail: error.to_string(),
            }
        })?;

        Ok(())
    }

    /// Shutdown the service.
    pub fn shutdown(&self) {
        // no persistent connection to shut down
    }

    /// Return the compiler for a workspace root.
    pub fn compiler_for_workspace_root(
        &self,
        root: &Path,
    ) -> Result<Arc<Compiler>, LanguageServiceError> {
        let workspace = self.workspace_for_root(root)?;
        Ok(workspace.compiler().clone())
    }

    /// Return the number of active workspace roots.
    pub fn workspace_root_count(&self) -> usize {
        self.workspaces_by_root.len()
    }

    /// Resolve or create a workspace for a root.
    pub(super) fn workspace_for_root(
        &self,
        root: &Path,
    ) -> Result<Arc<Session>, LanguageServiceError> {
        let root = root.to_path_buf();

        if !self.workspaces_by_root.contains_key(&root) {
            self.open_workspace_root(root.clone())?;
        }

        let session = self
            .workspaces_by_root
            .get(root.as_path())
            .map(|entry| Arc::clone(entry.value()))
            .ok_or(LanguageServiceError::RevisionNotTracked { root: root.clone() })?;

        if session.root() != root {
            return Err(LanguageServiceError::Internal {
                detail: format!(
                    "workspace root mismatch: expected {}, found {}",
                    root.display(),
                    session.root().display()
                ),
            });
        }

        Ok(session)
    }

    /// Resolve or create a workspace for a path.
    pub(super) fn workspace_for_path(
        &self,
        path: &Path,
    ) -> Result<Arc<Session>, LanguageServiceError> {
        let Some(root) = self.semantic_workspace_root_for_path(path)? else {
            return Err(LanguageServiceError::PathNotInWorkspace {
                path: path.to_path_buf(),
            });
        };

        self.workspace_for_root(&root)
    }

    /// Return the session for the workspace that owns a path.
    pub fn session_for_path(&self, path: &Path) -> Result<Arc<Session>, LanguageServiceError> {
        self.workspace_for_path(path)
    }

    /// Resolve the current semantic revision for the workspace that owns a path.
    pub fn revision_for_path(&self, path: &Path) -> Result<Revision, LanguageServiceError> {
        let root = self.workspace_root_for_path(path)?;
        self.revision_for_root(&root)
    }

    /// Resolve the current semantic revision for a workspace root.
    pub fn revision_for_root(&self, root: &Path) -> Result<Revision, LanguageServiceError> {
        let workspace = self.workspace_for_root(root)?;
        Ok(workspace.revision())
    }

    /// Execute a callback with workspace repository and compiler handles while holding the compile lock.
    pub fn with_workspace_for_root<T, F>(
        &self,
        root: &Path,
        callback: F,
    ) -> Result<T, LanguageServiceError>
    where
        F: FnOnce(Arc<Repository>, Arc<Compiler>) -> T,
    {
        let workspace = self.workspace_for_root(root)?;
        let _compile_guard = workspace.enter_mutation();

        Ok(callback(
            workspace.repository().clone(),
            workspace.compiler().clone(),
        ))
    }

    /// Build a workspace for a root.
    fn build_session(&self, root: PathBuf) -> Result<Session, LanguageServiceError> {
        let revision_ref = Ref::for_workspace_root(root.as_path());

        if self.repository.current(&revision_ref).is_err() {
            let repository_root_ref = Ref::for_workspace_root(self.repository.workspace_root());
            self.repository
                .fork_ref(&repository_root_ref, revision_ref.clone())
                .map_err(LanguageServiceError::from)?;
        }

        let compiler = Arc::new(Compiler::new(
            self.repository.clone(),
            self.compiler_execution_options.clone(),
        ));
        let linter = Arc::new(Linter::new(self.repository.clone()));

        let cwd = root.clone();

        Session::new(
            root,
            cwd,
            self.repository.clone(),
            revision_ref,
            self.overlay_fs.clone(),
            compiler,
            linter,
            self.session_event_handler.clone(),
        )
        .map_err(LanguageServiceError::from)
    }
}
