use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_source::{File, FileId, ModuleId, OverlayFileSystem, Uri};
use destack_workspace::{Ref, Repository, Revision};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::SessionError;

use super::state::SessionState;
use super::{ProvideId, SessionEvent, SessionEventHandler, SessionObservationHandler};

/// Monotonic id source for private session refs.
static NEXT_PRIVATE_SESSION_ID: AtomicU64 = AtomicU64::new(1);

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
    /// Overlay and open-file state for this root.
    pub(super) state: SessionState,
    /// Compiler for this root.
    pub(super) compiler: Arc<Compiler>,
    /// Linter for this root.
    pub(super) linter: Arc<Linter>,
    /// Serialize semantic access per root.
    pub(super) mutation_lock: RwLock<()>,
    /// Optional outer session event handler.
    pub(super) event_handler: Option<SessionEventHandler>,
    /// Optional outer session observation handler.
    pub(super) observation_handler: Option<SessionObservationHandler>,
    /// Threshold for slow artifact events when configured.
    pub(super) slow_artifact_threshold: Option<Duration>,
    /// Monotonic ids for provide attempts.
    pub(super) next_provide_id: AtomicU32,
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
            .field("state", &self.state)
            .field("compiler", &self.compiler)
            .field("linter", &self.linter)
            .field("event_handler", &self.event_handler.is_some())
            .field("observation_handler", &self.observation_handler.is_some())
            .field("slow_artifact_threshold", &self.slow_artifact_threshold)
            .finish_non_exhaustive()
    }
}

impl Session {
    /// Create a private session head rooted at one explicit revision.
    pub fn fork(
        root: PathBuf,
        cwd: PathBuf,
        repository: Arc<Repository>,
        revision: Revision,
        compiler: Arc<Compiler>,
        linter: Arc<Linter>,
        event_handler: Option<SessionEventHandler>,
        observation_handler: Option<SessionObservationHandler>,
    ) -> Result<Self, SessionError> {
        // create one private movable head for this session
        let session_id = NEXT_PRIVATE_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let head = Ref::new(format!("session:{}:{session_id}", root.display()));

        repository
            .point(&head, revision)
            .map_err(SessionError::from)?;

        Self::new(
            root,
            cwd,
            repository,
            head,
            None,
            compiler,
            linter,
            event_handler,
            observation_handler,
        )
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
        observation_handler: Option<SessionObservationHandler>,
    ) -> Result<Self, SessionError> {
        Ok(Self {
            root,
            cwd,
            repository,
            head,
            state: SessionState::new(overlay_fs),
            compiler,
            linter,
            mutation_lock: RwLock::new(()),
            event_handler,
            observation_handler,
            slow_artifact_threshold: capture_slow_artifact_threshold(),
            next_provide_id: AtomicU32::new(1),
        })
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

    /// Return the current semantic revision.
    pub fn revision(&self) -> Revision {
        self.repository
            .current(&self.head)
            .expect("session head should be tracked")
    }

    /// Return one mutable working revision for this session head.
    pub(crate) fn ensure_mutable_revision(&self) -> Result<Revision, SessionError> {
        let revision = self.revision();
        let revision_state = self
            .repository
            .revision(revision)
            .map_err(SessionError::from)?;

        if revision_state.is_mutable() {
            return Ok(revision);
        }

        let revision = self
            .repository
            .fork_mutable_revision(revision)
            .map_err(SessionError::from)?;

        self.point_revision(revision)?;

        Ok(revision)
    }

    /// Freeze the current session head revision to one immutable revision.
    pub(crate) fn freeze_revision(&self) -> Result<Revision, SessionError> {
        let revision = self.revision();
        let revision = self
            .repository
            .freeze_revision(revision)
            .map_err(SessionError::from)?;

        self.point_revision(revision)?;

        Ok(revision)
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

    /// Return the installed outer session observation handler.
    pub(crate) fn observation_handler(&self) -> Option<SessionObservationHandler> {
        self.observation_handler.clone()
    }

    /// Allocate the next provide attempt id.
    pub(crate) fn next_provide_id(&self) -> ProvideId {
        let provide_id = self.next_provide_id.fetch_add(1, Ordering::Relaxed);

        ProvideId(provide_id)
    }

    /// Return the configured slow artifact event threshold.
    pub(crate) fn slow_artifact_threshold(&self) -> Option<Duration> {
        self.slow_artifact_threshold
    }

    /// Point the session head at one explicit revision.
    fn point_revision(&self, revision: Revision) -> Result<Revision, SessionError> {
        self.repository
            .point(&self.head, revision)
            .map_err(SessionError::from)
    }

    /// Publish one staged semantic revision as the current session revision.
    pub(crate) fn publish_revision(&self, revision: Revision) -> Result<Revision, SessionError> {
        let revision = self
            .repository
            .freeze_revision(revision)
            .map_err(SessionError::from)?;

        self.point_revision(revision)
    }

    /// Return one tracked file id for a path in the current revision.
    pub fn file_id_for_path(&self, path: &Path) -> Result<Option<FileId>, SessionError> {
        let revision = self.revision();
        let file_id = self.repository.file_id_for_workspace_path(path);

        let file = self
            .repository
            .file(revision, file_id)
            .map_err(SessionError::from)?;

        Ok(file.map(|_| file_id))
    }

    /// Return true when the current semantic workspace contains this path.
    pub fn owns_semantic_path(&self, path: &Path) -> Result<bool, SessionError> {
        Ok(self.file_id_for_path(path)?.is_some())
    }

    /// Return one tracked file snapshot for a path when present.
    pub fn file_for_path(&self, path: &Path) -> Result<Option<(FileId, Arc<File>)>, SessionError> {
        let Some(file_id) = self.file_id_for_path(path)? else {
            return Ok(None);
        };
        let file = self
            .repository
            .file(self.revision(), file_id)
            .map_err(SessionError::from)?
            .ok_or(SessionError::FileIdNotTracked { file_id })?;

        Ok(Some((file_id, file)))
    }

    /// Return one tracked file snapshot by id.
    pub fn file_for_id(&self, file_id: FileId) -> Result<Arc<File>, SessionError> {
        let file = self
            .repository
            .file(self.revision(), file_id)
            .map_err(SessionError::from)?
            .ok_or(SessionError::FileIdNotTracked { file_id })?;

        Ok(file)
    }

    /// Track one open file by path.
    pub(crate) fn set_open_file(&self, path: &Path, uri: Uri, version: i32) {
        self.state.set_open_file(path, uri, version);
    }

    /// Set one overlay projection when available.
    pub(crate) fn set_overlay_for_path(&self, path: &Path, text: String) {
        self.state.set_overlay_for_path(path, text);
    }

    /// Remove one overlay projection when available.
    pub(crate) fn remove_overlay_for_path(&self, path: &Path) {
        self.state.remove_overlay_for_path(path);
    }

    /// Return true when a path is tracked as one open file.
    pub fn has_open_file_for_path(&self, path: &Path) -> bool {
        self.state.has_open_file_for_path(path)
    }

    /// Return one open file identity for a path.
    pub(crate) fn open_file_identity_for_path(&self, path: &Path) -> Option<(Uri, i32)> {
        self.state.open_file_identity_for_path(path)
    }

    /// Return one tracked open file snapshot for a path.
    pub fn open_file_for_path(&self, path: &Path) -> io::Result<Option<(Uri, i32, String)>> {
        self.state.open_file_for_path(path)
    }

    /// Return tracked overlay text for one open file when available.
    pub(crate) fn open_file_text_for_path(&self, path: &Path) -> io::Result<Option<String>> {
        self.state.open_file_text_for_path(path)
    }

    /// Return the tracked open files keyed by path.
    pub fn open_file_identities(&self) -> Vec<(PathBuf, Uri, i32)> {
        self.state.open_file_identities()
    }

    /// Stop tracking one open file and return its last known state.
    pub(crate) fn close_open_file(&self, path: &Path) -> Option<(Uri, i32)> {
        self.state.close_open_file(path)
    }

    /// Admit one module path into the current revision when needed.
    pub fn admit_module_for_path(&self, path: &Path) -> Result<ModuleId, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let revision = self.ensure_mutable_revision()?;
        let (revision, module_id) = self.admit_module_for_revision(revision, path)?;

        self.publish_revision(revision)?;

        Ok(module_id)
    }
}

/// Capture the slow artifact logging threshold from the process environment.
fn capture_slow_artifact_threshold() -> Option<Duration> {
    let value = std::env::var("DESTACK_SLOW_TASK_MS").ok()?;
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return None;
    }

    let millis = trimmed.parse::<u64>().ok()?;

    Some(Duration::from_millis(millis))
}
