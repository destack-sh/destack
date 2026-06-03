use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_source::{FileId, ModuleId};
use destack_workspace::{Ref, Repository, Revision};

use crate::executor::Executor;
use crate::{FileSystemSource, SessionError, SourceSnapshot};

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
    /// Return the default session worker count.
    pub fn default_worker_count() -> usize {
        thread::available_parallelism().map_or(1, usize::from)
    }

    /// Create a private session head rooted at one explicit revision.
    pub fn fork(
        root: PathBuf,
        cwd: PathBuf,
        repository: Arc<Repository>,
        head: Ref,
        revision: Revision,
        compiler: Arc<Compiler>,
        linter: Arc<Linter>,
        query: Arc<Query>,
        worker_count: usize,
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
            query,
            worker_count,
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
        query: Arc<Query>,
        worker_count: usize,
        event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, SessionError> {
        let state = Arc::new(SessionState::new(
            repository,
            compiler,
            linter,
            query,
            event_handler,
        ));

        Ok(Self {
            root,
            cwd,
            state: state.clone(),
            head,
            executor: Executor::new(state, worker_count)?,
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

    /// Return one session-root relative repository path.
    pub fn repository_path(&self, path: &Path) -> String {
        // direct root-relative path
        if let Ok(path) = path.strip_prefix(&self.root) {
            return self.normalize_repository_path(path);
        }

        // canonical root-relative path
        let repository = self.state.repository();
        let root = repository.file_system().canonicalize(&self.root);
        let canonical_path = repository.file_system().canonicalize(path);
        if let (Ok(root), Ok(canonical_path)) = (root, canonical_path)
            && let Ok(path) = canonical_path.strip_prefix(root)
        {
            return self.normalize_repository_path(path);
        }

        self.normalize_repository_path(path)
    }

    /// Return one session-root relative file id.
    pub fn file_id(&self, path: &Path) -> FileId {
        let repository_path = self.repository_path(path);

        FileId::from_logical_str(&repository_path)
    }

    /// Normalize one session repository path.
    fn normalize_repository_path(&self, path: impl AsRef<Path>) -> String {
        path.as_ref().to_string_lossy().replace('\\', "/")
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

    /// Return the query provider for this session.
    pub fn query(&self) -> Arc<Query> {
        self.state.query()
    }

    /// Load one filesystem module path into one ref when needed.
    pub fn load_module_from_fs(
        &self,
        reference: &Ref,
        path: &Path,
    ) -> Result<ModuleId, SessionError> {
        let repository = self.repository();
        let revision = self.revision(reference)?;

        // reuse already tracked modules
        let module_id = repository.module_id_for_path(revision, path)?;
        if let Some(module_id) = module_id {
            return Ok(module_id);
        }

        // read the requested filesystem source file
        let repository_path = self.repository_path(path);
        let source = FileSystemSource::new(repository.as_ref(), self.root());
        let Some(file) = source.file(Path::new(&repository_path))? else {
            return Err(SessionError::ModulePathNotLoadable {
                path: path.to_path_buf(),
                detail: "source file is not importable".to_string(),
            });
        };

        // apply the selected source file
        let snapshot = SourceSnapshot::from_file(file);
        let change = snapshot.change(repository.as_ref(), revision)?;
        let next_revision = repository.commit_change(revision, change)?;
        let _next_revision_pin = repository.pin(next_revision)?;

        // require the applied file to produce a module
        let module_id = repository.module_id_for_path(next_revision, path)?;
        let Some(module_id) = module_id else {
            return Err(SessionError::ModulePathNotLoadable {
                path: path.to_path_buf(),
                detail: "loaded source file did not produce a module".to_string(),
            });
        };

        // publish when the ref still points at the loaded base
        let was_published = repository.advance_ref(reference, revision, next_revision)?;
        if !was_published {
            let current = self.revision(reference)?;
            if let Some(module_id) = repository.module_id_for_path(current, path)? {
                return Ok(module_id);
            }

            return Err(SessionError::StaleRevision {
                reference: reference.clone(),
                expected: revision,
                current,
            });
        }

        Ok(module_id)
    }
}
