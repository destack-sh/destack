use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use dashmap::mapref::entry::Entry;
#[cfg(feature = "query")]
use destack_compiler::AnalyzeTask;
#[cfg(feature = "query")]
use destack_compiler::TaskOutcome;
use destack_compiler::{Compiler, CompilerOptions};
#[cfg(feature = "query")]
use destack_source::FileId;
use destack_workspace::{Program, Session};

use super::workspace::WorkspaceHandle;
use super::{LanguageService, LanguageServiceError, WorkspaceHandleId};

impl LanguageService {
    /// Create a local workspace service with explicit compiler options.
    pub fn with_options(
        session: Arc<Session>,
        roots: Vec<PathBuf>,
        compiler_options: CompilerOptions,
    ) -> Result<Self, LanguageServiceError> {
        // construct the service shell
        let workspace = Self {
            session,
            compiler_execution_options: compiler_options,
            handles_by_id: dashmap::DashMap::new(),
            handle_ids_by_root: dashmap::DashMap::new(),
            next_handle_id: std::sync::atomic::AtomicU64::new(1),
        };

        // register each workspace root
        for root in roots {
            workspace.open_workspace_root(root)?;
        }

        Ok(workspace)
    }

    /// Ensure the workspace root is opened.
    pub fn open_workspace_root(&self, root: PathBuf) -> Result<(), LanguageServiceError> {
        // ensure handle registration is atomic per root
        match self.handle_ids_by_root.entry(root.clone()) {
            Entry::Occupied(_) => {
                return Ok(());
            }
            Entry::Vacant(entry) => {
                // allocate and register a stable handle id
                let handle_id = self.next_handle_id.fetch_add(1, Ordering::Relaxed);
                let handle_id = WorkspaceHandleId(handle_id);

                // create a workspace handle for this root
                let handle = self.build_workspace_handle(handle_id, root.clone());
                self.handles_by_id.insert(handle_id, Arc::new(handle));
                entry.insert(handle_id);
            }
        }

        Ok(())
    }

    /// Close an opened workspace root.
    pub fn close_workspace_root(&self, root: &Path) -> Result<(), LanguageServiceError> {
        let root = root.to_path_buf();

        // ignore roots that are not open
        let Some((_, handle_id)) = self.handle_ids_by_root.remove(root.as_path()) else {
            return Ok(());
        };

        // remove cached workspace handle
        self.handles_by_id.remove(&handle_id);

        Ok(())
    }

    /// Return true when a workspace root handle is active.
    pub fn has_workspace_root(&self, root: &Path) -> bool {
        let root = root.to_path_buf();
        self.handle_ids_by_root.contains_key(root.as_path())
    }

    /// Remove an opened workspace root and report whether it existed.
    pub fn remove_workspace_root(&self, root: &Path) -> Result<bool, LanguageServiceError> {
        // capture presence before closing
        let existed = self.has_workspace_root(root);

        // close the root mapping
        self.close_workspace_root(root)?;

        Ok(existed)
    }

    /// Clear all cache entries for every workspace handle.
    pub fn clear_cache_all(&self) -> Result<(), LanguageServiceError> {
        // resolve the cache directory for this session
        let cache_dir = self.session.workspace_cache_dir();

        // return when there is no cache directory yet
        if !cache_dir.exists() {
            return Ok(());
        }

        // remove the full cache tree
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

    /// Return the compiler handle for a workspace root.
    pub fn compiler_for_workspace_root(
        &self,
        root: &Path,
    ) -> Result<Arc<Compiler>, LanguageServiceError> {
        let handle = self.workspace_handle_for_root(root)?;
        Ok(handle.compiler.clone())
    }

    /// Return the number of active workspace handles.
    pub fn workspace_handle_count(&self) -> usize {
        self.handles_by_id.len()
    }

    /// Resolve the workspace handle id for a path.
    #[cfg(feature = "query")]
    pub(crate) fn workspace_handle_id_for_path(
        &self,
        path: &Path,
    ) -> Result<WorkspaceHandleId, LanguageServiceError> {
        // resolve the owning root from session routing
        let Some(program) = self.session.find_program_for_path_maybe(path) else {
            return Err(LanguageServiceError::PathNotInWorkspace {
                path: path.to_path_buf(),
            });
        };
        let root = program.cwd.clone();

        self.workspace_handle_id_for_root(&root)
    }

    /// Resolve the workspace handle id for a root.
    pub fn workspace_handle_id_for_root(
        &self,
        root: &Path,
    ) -> Result<WorkspaceHandleId, LanguageServiceError> {
        let root = root.to_path_buf();

        // return early when the root is already opened
        if let Some(handle_id) = self.handle_ids_by_root.get(root.as_path()) {
            return Ok(*handle_id.value());
        }

        // lazily register the root when not opened yet
        self.open_workspace_root(root.clone())?;

        // resolve the newly registered handle id
        self.handle_ids_by_root
            .get(&root)
            .map(|entry| *entry.value())
            .ok_or(LanguageServiceError::WorkspaceHandleMissingAfterOpen { root })
    }

    /// Resolve a workspace handle by id.
    pub(super) fn workspace_handle_for_id(
        &self,
        handle: WorkspaceHandleId,
    ) -> Result<Arc<WorkspaceHandle>, LanguageServiceError> {
        self.handles_by_id
            .get(&handle)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or(LanguageServiceError::UnknownWorkspaceHandle { handle })
    }

    /// Resolve or create a workspace handle for a root.
    pub(super) fn workspace_handle_for_root(
        &self,
        root: &Path,
    ) -> Result<Arc<WorkspaceHandle>, LanguageServiceError> {
        let root = root.to_path_buf();
        let handle_id = self.workspace_handle_id_for_root(&root)?;
        let handle = self.workspace_handle_for_id(handle_id)?;

        // ensure the root to handle mapping has not drifted
        if handle.root != root {
            return Err(LanguageServiceError::Internal {
                detail: format!(
                    "workspace handle root mismatch: expected {}, found {}",
                    root.display(),
                    handle.root.display(),
                ),
            });
        }

        // ensure the id to handle mapping has not drifted
        if handle.id != handle_id {
            return Err(LanguageServiceError::Internal {
                detail: format!(
                    "workspace handle id mismatch: expected {handle_id:?}, found {:?}",
                    handle.id,
                ),
            });
        }

        Ok(handle)
    }

    /// Resolve or create a workspace handle for a path.
    pub(super) fn workspace_handle_for_path(
        &self,
        path: &Path,
    ) -> Result<Arc<WorkspaceHandle>, LanguageServiceError> {
        // resolve the owning root for this path
        let Some(program) = self.session.find_program_for_path_maybe(path) else {
            return Err(LanguageServiceError::PathNotInWorkspace {
                path: path.to_path_buf(),
            });
        };

        self.workspace_handle_for_root(&program.cwd)
    }

    /// Return the session backing this service.
    #[cfg(feature = "query")]
    pub(crate) fn session_ref(&self) -> &Session {
        self.session.as_ref()
    }

    /// Increment the semantic revision for a workspace root.
    pub(crate) fn bump_revision_for_root(&self, root: &Path) -> Result<u64, LanguageServiceError> {
        let root = root.to_path_buf();
        let handle_id = self
            .handle_ids_by_root
            .get(root.as_path())
            .map(|entry| *entry.value())
            .ok_or(LanguageServiceError::RevisionNotTracked { root })?;
        let handle = self.workspace_handle_for_id(handle_id)?;

        Ok(handle.bump_revision())
    }

    /// Resolve the current semantic revision for the workspace that owns a path.
    pub fn revision_for_path(&self, path: &Path) -> Result<u64, LanguageServiceError> {
        // resolve the workspace handle for this path
        let handle = self.workspace_handle_id_for_path(path)?;
        self.revision_for_handle(handle)
    }

    /// Resolve the current semantic revision for a workspace handle.
    pub fn revision_for_handle(
        &self,
        handle: WorkspaceHandleId,
    ) -> Result<u64, LanguageServiceError> {
        let handle = self.workspace_handle_for_id(handle)?;
        Ok(handle.revision())
    }

    /// Validate module semantics for file scoped queries.
    #[cfg(feature = "query")]
    pub(crate) fn validate_semantic_query_module(
        &self,
        program: &Program,
        file_id: FileId,
    ) -> Result<Option<TaskOutcome>, LanguageServiceError> {
        // resolve the module and profile for this file
        let session = self.session.as_ref();
        let Some(module_id) = session.modules.get_id_by_file_id(file_id) else {
            return Ok(None);
        };
        let profile_id = session.default_profile_for_module(module_id);

        // serialize validation against this root compiler
        let handle = self.workspace_handle_for_root(&program.cwd)?;
        let _compile_guard = handle.compile_lock.lock();

        // run module validation and return the typed outcome
        let module = handle.compiler.module_stamp(module_id);
        let profile = handle.compiler.profile_stamp(profile_id);
        let analyze_task = AnalyzeTask::AnalyzeModuleValidate { module, profile };
        let outcome = handle.compiler.run_task(analyze_task);

        Ok(Some(outcome))
    }

    /// Execute a callback with workspace program and compiler handles while holding the compile lock.
    pub fn with_workspace_handles_for_path<T, F>(
        &self,
        path: &Path,
        callback: F,
    ) -> Result<T, LanguageServiceError>
    where
        F: FnOnce(Arc<Program>, Arc<Compiler>) -> T,
    {
        // resolve and lock the owning workspace handle
        let handle = self.workspace_handle_for_path(path)?;
        let _compile_guard = handle.compile_lock.lock();

        // run the callback with shared program and compiler handles
        Ok(callback(handle.program.clone(), handle.compiler.clone()))
    }

    /// Build a workspace handle for a root and id.
    fn build_workspace_handle(&self, id: WorkspaceHandleId, root: PathBuf) -> WorkspaceHandle {
        // resolve or create the program for this root
        let program = self.session.get_or_create_program(root.clone());

        // create a compiler bound to this program
        let compiler = Arc::new(Compiler::new(
            self.session.clone(),
            program.clone(),
            self.compiler_execution_options.clone(),
        ));

        WorkspaceHandle::new(id, root, program, compiler)
    }
}
