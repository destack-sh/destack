use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use dashmap::mapref::entry::Entry;
use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, TaskOutcome};
use destack_source::FileId;
use destack_workspace::{Program, Session};

use super::workspace::ProgramHandle;
use super::{WorkspaceHandleId, WorkspaceService, WorkspaceServiceError};

impl WorkspaceService {
    /// Create a local workspace service for the provided roots.
    pub fn new(session: Arc<Session>, roots: Vec<PathBuf>) -> Result<Self, WorkspaceServiceError> {
        Self::with_options(session, roots, CompilerOptions::default())
    }

    /// Create a local workspace service with explicit compiler options.
    pub fn with_options(
        session: Arc<Session>,
        roots: Vec<PathBuf>,
        compiler_options: CompilerOptions,
    ) -> Result<Self, WorkspaceServiceError> {
        // construct the service shell
        let workspace = Self {
            session,
            compiler_options,
            program_handles: dashmap::DashMap::new(),
            handles_by_root: dashmap::DashMap::new(),
            roots_by_handle: dashmap::DashMap::new(),
            next_handle_id: std::sync::atomic::AtomicU64::new(1),
        };

        // register each workspace root
        for root in roots {
            workspace.open_workspace_root(root)?;
        }

        Ok(workspace)
    }

    /// Ensure the workspace root is opened.
    pub fn open_workspace_root(&self, root: PathBuf) -> Result<(), WorkspaceServiceError> {
        // ensure handle registration is atomic per root
        match self.handles_by_root.entry(root.clone()) {
            Entry::Occupied(_) => return Ok(()),
            Entry::Vacant(entry) => {
                // ensure a program exists for this root
                self.session.get_or_create_program(root.clone());

                // allocate and register a stable handle id
                let handle_id = self.next_handle_id.fetch_add(1, Ordering::Relaxed);
                let handle = WorkspaceHandleId(handle_id);
                entry.insert(handle);
                self.roots_by_handle.insert(handle, root);
            }
        }

        Ok(())
    }

    /// Close an opened workspace root.
    pub fn close_workspace_root(&self, root: &Path) -> Result<(), WorkspaceServiceError> {
        // ignore roots that are not open
        let Some((_, handle)) = self.handles_by_root.remove(root) else {
            return Ok(());
        };

        // remove reverse mappings and cached handles
        self.roots_by_handle.remove(&handle);
        self.program_handles.remove(root);

        Ok(())
    }

    /// Return true when a workspace root handle is active.
    pub fn has_workspace_root(&self, root: &Path) -> bool {
        self.handles_by_root.contains_key(root)
    }

    /// Remove an opened workspace root and report whether it existed.
    pub fn remove_workspace_root(&self, root: &Path) -> Result<bool, WorkspaceServiceError> {
        // capture presence before closing
        let existed = self.has_workspace_root(root);

        // close the root mapping
        self.close_workspace_root(root)?;

        Ok(existed)
    }

    /// Clear all cache entries for every workspace handle.
    pub fn clear_cache_all(&self) -> Result<(), WorkspaceServiceError> {
        // resolve the cache directory for this session
        let cache_dir = self.session.workspace_cache_dir();

        // return when there is no cache directory yet
        if !cache_dir.exists() {
            return Ok(());
        }

        // remove the full cache tree
        std::fs::remove_dir_all(&cache_dir).map_err(|error| {
            WorkspaceServiceError::CacheClearFailed {
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
    pub fn compiler_for_root(&self, root: &Path) -> Arc<Compiler> {
        let handle = self.program_handle_for_root(root);
        handle.compiler.clone()
    }

    /// Return the number of active program handles.
    pub fn program_handle_count(&self) -> usize {
        self.program_handles.len()
    }

    /// Resolve the workspace handle for a path.
    pub(crate) fn handle_for_path(
        &self,
        path: &Path,
    ) -> Result<WorkspaceHandleId, WorkspaceServiceError> {
        // resolve the owning root from session routing
        let program = self.session.find_program_for_path(path);
        let root = program.cwd.clone();

        // use the existing handle when present
        if let Some(handle) = self.handles_by_root.get(&root) {
            return Ok(*handle.value());
        }

        // lazily register the root when not opened yet
        self.open_workspace_root(root.clone())?;

        // fetch the newly registered handle
        self.handles_by_root
            .get(&root)
            .map(|entry| *entry.value())
            .ok_or(WorkspaceServiceError::WorkspaceHandleMissingAfterOpen { root })
    }

    /// Resolve the workspace handle for a root.
    pub fn handle_for_root(&self, root: &Path) -> Result<WorkspaceHandleId, WorkspaceServiceError> {
        // use the existing handle when present
        if let Some(handle) = self.handles_by_root.get(root) {
            return Ok(*handle.value());
        }

        // lazily register the root when not opened yet
        let root = root.to_path_buf();
        self.open_workspace_root(root.clone())?;

        // fetch the newly registered handle
        self.handles_by_root
            .get(&root)
            .map(|entry| *entry.value())
            .ok_or(WorkspaceServiceError::WorkspaceHandleMissingAfterOpen { root })
    }

    /// Resolve or create a program handle for a root.
    pub(super) fn program_handle_for_root(&self, root: &Path) -> Arc<ProgramHandle> {
        // ensure handle creation is atomic per root
        match self.program_handles.entry(root.to_path_buf()) {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => {
                // resolve the program for this root
                let program = self.session.get_or_create_program(root.to_path_buf());

                // create a compiler bound to this program
                let compiler = Arc::new(Compiler::new(
                    self.session.clone(),
                    program.clone(),
                    self.compiler_options.clone(),
                ));

                // create and store the synchronized handle
                let handle = Arc::new(ProgramHandle {
                    program,
                    compiler,
                    compile_lock: parking_lot::Mutex::new(()),
                });
                let inserted = entry.insert(handle);
                Arc::clone(inserted.value())
            }
        }
    }

    /// Resolve or create a program handle for a path.
    pub(super) fn program_handle_for_path(&self, path: &Path) -> Arc<ProgramHandle> {
        // resolve the path owner and delegate to root lookup
        let program = self.session.find_program_for_path(path);
        self.program_handle_for_root(&program.cwd)
    }

    /// Return the session backing this service.
    pub(crate) fn session_ref(&self) -> &Session {
        self.session.as_ref()
    }

    /// Resolve a root path for a workspace handle.
    pub(crate) fn root_for_handle(
        &self,
        handle: WorkspaceHandleId,
    ) -> Result<PathBuf, WorkspaceServiceError> {
        self.roots_by_handle
            .get(&handle)
            .map(|entry| entry.value().clone())
            .ok_or(WorkspaceServiceError::UnknownWorkspaceHandle { handle })
    }

    /// Resolve or create a program for a workspace root.
    pub(crate) fn program_for_root(&self, root: &Path) -> Arc<Program> {
        self.session.get_or_create_program(root.to_path_buf())
    }

    /// Validate module semantics for file scoped queries.
    pub(crate) fn validate_semantic_query_module(
        &self,
        program: &Program,
        file_id: FileId,
    ) -> Option<TaskOutcome> {
        // resolve the module and profile for this file
        let session = self.session.as_ref();
        let module_id = session.modules.get_id_by_file_id(file_id)?;
        let profile_id = session.default_profile_for_module(module_id);

        // serialize validation against this root compiler
        let handle = self.program_handle_for_root(&program.cwd);
        let _compile_guard = handle.compile_lock.lock();

        // run module validation and return the typed outcome
        let module = handle.compiler.module_stamp(module_id);
        let profile = handle.compiler.profile_stamp(profile_id);
        let analyze_task = AnalyzeTask::AnalyzeModuleValidate { module, profile };
        let outcome = handle.compiler.run_task(analyze_task);

        Some(outcome)
    }

    /// Execute a callback with program and compiler handles while holding the compile lock.
    pub fn with_program_for_path<T, F>(&self, path: &Path, callback: F) -> T
    where
        F: FnOnce(Arc<Program>, Arc<Compiler>) -> T,
    {
        // resolve and lock the owning program handle
        let handle = self.program_handle_for_path(path);
        let _compile_guard = handle.compile_lock.lock();

        // run the callback with shared program and compiler handles
        callback(handle.program.clone(), handle.compiler.clone())
    }
}
