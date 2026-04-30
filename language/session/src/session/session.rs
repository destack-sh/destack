use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_source::{File, FileId, ModuleId, OverlayFileSystem, Uri};
use destack_workspace::{Ref, Repository, Revision};
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

use crate::SessionError;
use crate::r#loop::SessionLoop;

use super::{FileUpdate, OpenFile, SessionEventHandler, SessionState};

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
    pub(crate) r#loop: Arc<SessionLoop>,
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
            .field("loop", &self.r#loop)
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

    /// Create a new live session for one source root.
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
        let worker_limit = compiler.options.workers as usize;
        let state = Arc::new(SessionState::new(
            repository,
            compiler,
            linter,
            overlay_fs,
            event_handler,
        ));

        Self {
            root,
            cwd,
            state: state.clone(),
            head,
            r#loop: SessionLoop::with_worker_limit(state, worker_limit),
        }
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

    /// Return one tracked file id for a path in a revision.
    pub fn file_id_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, SessionError> {
        let repository = self.repository();
        let file_id = repository.file_id_for_workspace_path(path);

        let file = repository
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
            .repository()
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
            .repository()
            .file(revision, file_id)
            .map_err(SessionError::from)?
            .ok_or(SessionError::FileIdNotTracked { file_id })?;

        Ok(file)
    }

    /// Track one open file by path.
    pub(crate) fn track_open_file(&self, path: &Path, uri: Uri) {
        self.state.overlay().track_file(path, uri);
    }

    /// Set overlay text for one open file when available.
    pub(crate) fn set_open_file_text(&self, path: &Path, text: String) {
        self.state.overlay().set_file_text(path, text);
    }

    /// Remove overlay text for one open file when available.
    pub(crate) fn remove_open_file_text(&self, path: &Path) {
        self.state.overlay().remove_file_text(path);
    }

    /// Return true when a path is tracked as one open file.
    pub fn contains_open_file(&self, path: &Path) -> bool {
        self.state.overlay().contains_file(path)
    }

    /// Return one open file for a path.
    pub(crate) fn tracked_open_file(&self, path: &Path) -> Option<OpenFile> {
        self.state.overlay().file(path)
    }

    /// Return one tracked open file for a path.
    pub fn open_file(&self, path: &Path) -> io::Result<Option<(Uri, String)>> {
        self.state.overlay().open_file(path)
    }

    /// Return tracked overlay text for one open file when available.
    pub(crate) fn open_file_text(&self, path: &Path) -> io::Result<Option<String>> {
        self.state.overlay().file_text(path)
    }

    /// Return the tracked open files keyed by path.
    pub fn open_files(&self) -> Vec<(PathBuf, Uri)> {
        self.state.overlay().files()
    }

    /// Stop tracking one open file and return its last known state.
    pub(crate) fn untrack_open_file(&self, path: &Path) -> Option<OpenFile> {
        self.state.overlay().untrack_file(path)
    }

    /// Import one filesystem module path into one ref when needed.
    pub fn load_module_from_fs(
        &self,
        reference: &Ref,
        path: &Path,
    ) -> Result<ModuleId, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let revision = self.revision(reference)?;
        let (revision, module_id) = self.import_module_file(revision, path)?;

        self.set_ref(reference, revision)?;

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
