use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_source::{File, FileId, ModuleId, OverlayFileSystem, Uri};
use destack_workspace::{Ref, Repository, Revision};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::SessionError;
use crate::r#loop::SessionLoop;
use crate::repository::{OpenFileOverlay, canonical_path_or_original};

use super::{FileUpdate, SessionEvent, SessionEventHandler, SessionRunId};

/// One live line of work over one repository workspace root.
pub struct Session {
    /// Root path for this workspace.
    pub(super) root: PathBuf,
    /// Working directory for this live session.
    pub(super) cwd: PathBuf,
    /// Repository that owns this workspace root.
    pub(super) repository: Arc<Repository>,
    /// Moving repository ref for this workspace root.
    pub(super) head: Ref,
    /// Open file identities and overlay text for this root.
    pub(super) open_file_overlay: OpenFileOverlay,
    /// Compiler for this root.
    pub(super) compiler: Arc<Compiler>,
    /// Linter for this root.
    pub(super) linter: Arc<Linter>,
    /// Artifact executor for this live session.
    pub(crate) r#loop: Arc<SessionLoop>,
    /// Serialize semantic access per root.
    pub(super) mutation_lock: RwLock<()>,
    /// Optional outer session event handler.
    pub(super) event_handler: Option<SessionEventHandler>,
    /// Monotonic ids for session runs.
    pub(super) next_session_run_id: AtomicU32,
}

impl std::fmt::Debug for Session {
    /// Format the visible session state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("root", &self.root)
            .field("cwd", &self.cwd)
            .field("repository", &self.repository)
            .field("head", &self.head)
            .field("open_file_overlay", &self.open_file_overlay)
            .field("compiler", &self.compiler)
            .field("linter", &self.linter)
            .field("loop", &self.r#loop)
            .field("event_handler", &self.event_handler.is_some())
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
        event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, SessionError> {
        // bind the explicit private ref to its base revision
        repository
            .set_ref(&head, revision)
            .map_err(SessionError::from)?;

        Ok(Self::new(
            root,
            cwd,
            repository,
            head,
            None,
            compiler,
            linter,
            event_handler,
        ))
    }

    /// Create a new live session for one workspace root.
    pub fn new(
        root: PathBuf,
        cwd: PathBuf,
        repository: Arc<Repository>,
        head: Ref,
        overlay_fs: Option<Arc<OverlayFileSystem>>,
        compiler: Arc<Compiler>,
        linter: Arc<Linter>,
        event_handler: Option<SessionEventHandler>,
    ) -> Self {
        Self {
            root,
            cwd,
            repository,
            head,
            open_file_overlay: OpenFileOverlay::new(overlay_fs),
            r#loop: Arc::new(SessionLoop::with_worker_limit(
                compiler.options.workers as usize,
            )),
            compiler,
            linter,
            mutation_lock: RwLock::new(()),
            event_handler,
            next_session_run_id: AtomicU32::new(1),
        }
    }

    /// Return the workspace root for this session.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the working directory for this session.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Enter a coherent read section for this session.
    pub fn enter_query(&self) -> RwLockReadGuard<'_, ()> {
        self.mutation_lock.read()
    }

    /// Enter a mutation section for this session.
    pub fn enter_mutation(&self) -> RwLockWriteGuard<'_, ()> {
        self.mutation_lock.write()
    }

    /// Return the repository for this session.
    pub fn repository(&self) -> Arc<Repository> {
        self.repository.clone()
    }

    /// Return the default session ref.
    pub fn head(&self) -> &Ref {
        &self.head
    }

    /// Return the revision currently bound to one ref.
    pub fn revision(&self, reference: &Ref) -> Result<Revision, SessionError> {
        self.repository
            .current(reference)
            .map_err(SessionError::from)
    }

    /// Return the compiler for this session.
    pub fn compiler(&self) -> Arc<Compiler> {
        self.compiler.clone()
    }

    /// Return the linter for this session.
    pub fn linter(&self) -> Arc<Linter> {
        self.linter.clone()
    }

    /// Emit one outer session event when a handler is installed.
    pub(crate) fn emit_event(&self, event: SessionEvent) {
        if let Some(handler) = &self.event_handler {
            handler(event);
        }
    }

    /// Allocate the next session run id.
    pub(crate) fn next_session_run_id(&self) -> SessionRunId {
        let run_id = self.next_session_run_id.fetch_add(1, Ordering::Relaxed);

        SessionRunId(run_id)
    }

    /// Commit one staged semantic revision to one ref.
    pub(crate) fn commit_revision(
        &self,
        reference: &Ref,
        revision: Revision,
    ) -> Result<Revision, SessionError> {
        self.repository
            .set_ref(reference, revision)
            .map_err(SessionError::from)
    }

    /// Return one tracked file id for a path in a revision.
    pub fn file_id_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, SessionError> {
        let file_id = self.repository.file_id_for_workspace_path(path);

        let file = self
            .repository
            .file(revision, file_id)
            .map_err(SessionError::from)?;

        Ok(file.map(|_| file_id))
    }

    /// Return true when a revision contains this semantic path.
    pub fn owns_semantic_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<bool, SessionError> {
        Ok(self.file_id_for_path(revision, path)?.is_some())
    }

    /// Return one tracked file for a path when present.
    pub fn file_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<(FileId, Arc<File>)>, SessionError> {
        let Some(file_id) = self.file_id_for_path(revision, path)? else {
            return Ok(None);
        };
        let file = self
            .repository
            .file(revision, file_id)
            .map_err(SessionError::from)?
            .ok_or(SessionError::FileIdNotTracked { file_id })?;

        Ok(Some((file_id, file)))
    }

    /// Return one tracked file by id.
    pub fn file_for_id(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Arc<File>, SessionError> {
        let file = self
            .repository
            .file(revision, file_id)
            .map_err(SessionError::from)?
            .ok_or(SessionError::FileIdNotTracked { file_id })?;

        Ok(file)
    }

    /// Track one open file by path.
    pub(crate) fn track_open_file(&self, path: &Path, uri: Uri, version: i32) {
        let path = canonical_path_or_original(path);

        self.open_file_overlay
            .track_open_file(path.as_path(), uri, version);
    }

    /// Set one overlay projection when available.
    pub(crate) fn set_overlay_for_path(&self, path: &Path, text: String) {
        let path = canonical_path_or_original(path);

        self.open_file_overlay
            .set_overlay_for_path(path.as_path(), text);
    }

    /// Remove one overlay projection when available.
    pub(crate) fn remove_overlay_for_path(&self, path: &Path) {
        let path = canonical_path_or_original(path);

        self.open_file_overlay
            .remove_overlay_for_path(path.as_path());
    }

    /// Return true when a path is tracked as one open file.
    pub fn contains_open_file(&self, path: &Path) -> bool {
        let path = canonical_path_or_original(path);

        self.open_file_overlay.contains_open_file(path.as_path())
    }

    /// Return one open file identity for a path.
    pub(crate) fn open_file_identity(&self, path: &Path) -> Option<(Uri, i32)> {
        let path = canonical_path_or_original(path);

        self.open_file_overlay.open_file_identity(path.as_path())
    }

    /// Return one tracked open file for a path.
    pub fn open_file(&self, path: &Path) -> io::Result<Option<(Uri, i32, String)>> {
        let path = canonical_path_or_original(path);

        self.open_file_overlay.open_file(path.as_path())
    }

    /// Return tracked overlay text for one open file when available.
    pub(crate) fn open_file_text(&self, path: &Path) -> io::Result<Option<String>> {
        let path = canonical_path_or_original(path);

        self.open_file_overlay.open_file_text(path.as_path())
    }

    /// Return the tracked open files keyed by path.
    pub fn open_file_identities(&self) -> Vec<(PathBuf, Uri, i32)> {
        self.open_file_overlay.open_file_identities()
    }

    /// Stop tracking one open file and return its last known state.
    pub(crate) fn untrack_open_file(&self, path: &Path) -> Option<(Uri, i32)> {
        let path = canonical_path_or_original(path);

        self.open_file_overlay.untrack_open_file(path.as_path())
    }

    /// Load one module path into one ref when needed.
    pub fn load_module(&self, reference: &Ref, path: &Path) -> Result<ModuleId, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let revision = self.revision(reference)?;
        let (revision, module_id) = self.load_module_revision(revision, path)?;

        self.commit_revision(reference, revision)?;

        Ok(module_id)
    }
}

/// One committed session ref movement.
#[derive(Debug, Clone)]
pub struct SessionChange {
    /// The repository ref that moved.
    pub reference: Ref,
    /// The revision the ref pointed at before the change.
    pub before: Revision,
    /// The revision the ref points at after the change.
    pub after: Revision,
    /// File updates visible to session consumers.
    pub files: Vec<FileUpdate>,
}
