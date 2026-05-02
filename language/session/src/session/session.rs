use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_source::ModuleId;
use destack_workspace::{Ref, Repository, Revision};
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

use crate::executor::Executor;
use crate::{FileSystemSource, RepositoryChange, RepositorySource, SessionError};

use super::{SessionEventHandler, SessionState};

/// Live source root backed by one default moving ref.
pub struct Session {
    /// Root path for source files owned by this session.
    pub(super) root: PathBuf,
    /// Working directory for this live session.
    pub(super) cwd: PathBuf,
    /// Shared session state for providers and workers.
    pub(crate) state: Arc<SessionState>,
    /// Default moving repository ref for this source root.
    pub(super) head: Ref,
    /// Artifact executor for this live session.
    pub(crate) executor: Arc<Executor>,
}

impl std::fmt::Debug for Session {
    /// Format the visible session state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("root", &self.root)
            .field("cwd", &self.cwd)
            .field("state", &self.state)
            .field("head", &self.head)
            .field("executor", &self.executor)
            .finish_non_exhaustive()
    }
}

impl Session {
    /// Create a private session head rooted at one explicit revision.
    pub fn fork(
        root: PathBuf,
        cwd: PathBuf,
        repository: Arc<Repository>,
        head: Ref,
        revision: Revision,
        compiler: Arc<Compiler>,
        linter: Arc<Linter>,
        worker_limit: usize,
        event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, SessionError> {
        // bind the explicit private ref to its base revision
        repository
            .set_ref(&head, revision)
            .map_err(SessionError::from)?;

        Self::new(
            root,
            cwd,
            repository,
            head,
            compiler,
            linter,
            worker_limit,
            event_handler,
        )
    }

    /// Create a new live session for one source root.
    pub fn new(
        root: PathBuf,
        cwd: PathBuf,
        repository: Arc<Repository>,
        head: Ref,
        compiler: Arc<Compiler>,
        linter: Arc<Linter>,
        worker_limit: usize,
        event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, SessionError> {
        let state = Arc::new(SessionState::new(
            repository,
            compiler,
            linter,
            event_handler,
        ));

        Ok(Self {
            root,
            cwd,
            state: state.clone(),
            head,
            executor: Executor::with_worker_limit(state, worker_limit)?,
        })
    }

    /// Return the source root for this session.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the working directory for this session.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Enter a coherent read section for this session.
    pub fn enter_query(&self) -> RwLockReadGuard<'_, ()> {
        self.state.enter_query()
    }

    /// Enter a mutation section for this session.
    pub fn enter_mutation(&self) -> RwLockWriteGuard<'_, ()> {
        self.state.enter_mutation()
    }

    /// Return the repository for this session.
    pub fn repository(&self) -> Arc<Repository> {
        self.state.repository()
    }

    /// Return the default session ref.
    pub fn head(&self) -> &Ref {
        &self.head
    }

    /// Return the revision currently bound to one ref.
    pub fn revision(&self, reference: &Ref) -> Result<Revision, SessionError> {
        self.state
            .repository()
            .current(reference)
            .map_err(SessionError::from)
    }

    /// Return the compiler for this session.
    pub fn compiler(&self) -> Arc<Compiler> {
        self.state.compiler()
    }

    /// Return the linter for this session.
    pub fn linter(&self) -> Arc<Linter> {
        self.state.linter()
    }

    /// Set one ref to an existing revision.
    pub(crate) fn set_ref(
        &self,
        reference: &Ref,
        revision: Revision,
    ) -> Result<Revision, SessionError> {
        self.state
            .repository()
            .set_ref(reference, revision)
            .map_err(SessionError::from)
    }

    /// Load one filesystem module path into one ref when needed.
    pub fn load_module_from_fs(
        &self,
        reference: &Ref,
        path: &Path,
    ) -> Result<ModuleId, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let repository = self.repository();
        let revision = self.revision(reference)?;

        // reuse already tracked modules
        let module_id = repository.module_id_for_path(revision, path)?;
        if let Some(module_id) = module_id {
            return Ok(module_id);
        }

        // read the requested filesystem source file
        let mut source = FileSystemSource::new(repository.as_ref(), self.root());
        let Some(file) = source.get(path)? else {
            return Err(SessionError::ResolvePathFailed {
                path: path.to_path_buf(),
                detail: "source file is not importable".to_string(),
            });
        };

        // apply the selected source file
        let change = RepositoryChange::from_files(repository.as_ref(), revision, &source, [file])?;
        let revision = change.apply(repository.as_ref(), revision)?;

        // require the applied file to produce a module
        let module_id = repository.module_id_for_path(revision, path)?;
        let Some(module_id) = module_id else {
            return Err(SessionError::ResolvePathFailed {
                path: path.to_path_buf(),
                detail: "loaded source file did not produce a module".to_string(),
            });
        };

        // publish the new revision only after validation
        self.set_ref(reference, revision)?;

        Ok(module_id)
    }
}
