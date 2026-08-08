use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_repository::{Ref, Revision};
use destack_session::{ArtifactPriority, ArtifactRun, Session};
use destack_source::Edit;
use parking_lot::Mutex;

use crate::diagnostic::Error;
use crate::file::{FileOperation, normalize_path};
use crate::{Watch, WatchState};

use super::LocalWorkspace;

/// One opened source root.
pub(crate) struct WorkspaceRoot {
    /// The live source session.
    pub(crate) session: Arc<Session>,
    /// Serializes source changes and open document state.
    pub(crate) writes: Mutex<()>,
    /// Semantic watch state.
    pub(crate) watch: Arc<Mutex<WatchState>>,
    /// Latest proactive editor artifact run.
    background_run: Mutex<Option<ArtifactRun>>,
}

impl std::fmt::Debug for WorkspaceRoot {
    /// Format the opened root state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceRoot")
            .field("session", &self.session)
            .field("writes", &self.writes)
            .field("watch", &self.watch)
            .field(
                "background_revision",
                &self
                    .background_run
                    .lock()
                    .as_ref()
                    .map(ArtifactRun::revision),
            )
            .finish()
    }
}

impl WorkspaceRoot {
    /// Open one semantic watch at the current root revision.
    pub(crate) fn watch(&self) -> Result<Watch, Error> {
        let _write = self.writes.lock();
        let revision = self.session.revision(self.session.head())?;

        Watch::new(
            self.session.root().to_path_buf(),
            revision,
            self.watch.clone(),
        )
    }

    /// Schedule proactive editor artifacts for one revision.
    pub(crate) fn schedule_background(&self, revision: Revision, artifacts: &[ArtifactKey]) {
        let run = (!artifacts.is_empty()).then(|| {
            self.session
                .schedule_artifacts(revision, artifacts, ArtifactPriority::Background)
        });
        let previous = std::mem::replace(&mut *self.background_run.lock(), run);

        // cancel obsolete work after publishing its replacement
        drop(previous);
    }
}

impl LocalWorkspace {
    /// Terminate semantic subscriptions after one host watch failure.
    pub fn fail_watch(&self, root: &Path, detail: String) -> Result<(), Error> {
        let root = self.workspace_root(root)?;
        let _write = root.writes.lock();
        root.watch.lock().fail(detail);

        Ok(())
    }

    /// Resolve one source path within one exact opened root.
    pub(crate) fn resolve_path(&self, root: &WorkspaceRoot, path: &Path) -> Result<PathBuf, Error> {
        // anchor relative paths at the requested root
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.session.root().join(path)
        };

        // require the resolved path to belong to that exact root
        let Some(owner) = self.resolve_configured_root(&path) else {
            return Err(Error::PathNotInRoot { path });
        };
        if owner != root.session.root() {
            return Err(Error::PathNotInRoot { path });
        }

        Ok(path)
    }

    /// Resolve every path in one editor file operation.
    pub(crate) fn resolve_operation(
        &self,
        root: &WorkspaceRoot,
        mut operation: FileOperation,
    ) -> Result<FileOperation, Error> {
        match &mut operation {
            FileOperation::OpenText { path, .. }
            | FileOperation::OpenBytes { path, .. }
            | FileOperation::ChangeText { path, .. }
            | FileOperation::ChangeBytes { path, .. }
            | FileOperation::PatchText { path, .. }
            | FileOperation::SaveText { path, .. }
            | FileOperation::SaveBytes { path, .. }
            | FileOperation::Close { path }
            | FileOperation::WriteText { path, .. }
            | FileOperation::WriteBytes { path, .. }
            | FileOperation::Remove { path } => *path = self.resolve_path(root, path)?,
            FileOperation::Move { from, to } => {
                *from = self.resolve_path(root, from)?;
                *to = self.resolve_path(root, to)?;
            }
        }

        Ok(operation)
    }

    /// Resolve every path in one source edit.
    pub(crate) fn resolve_edit(&self, root: &WorkspaceRoot, mut edit: Edit) -> Result<Edit, Error> {
        match &mut edit {
            Edit::SetText { path, .. }
            | Edit::EditText { path, .. }
            | Edit::SetBytes { path, .. }
            | Edit::Remove { path } => *path = self.resolve_path(root, path)?,
            Edit::Move { from, to } => {
                *from = self.resolve_path(root, from)?;
                *to = self.resolve_path(root, to)?;
            }
        }

        Ok(edit)
    }

    /// Resolve the configured root that owns a path.
    pub(super) fn resolve_configured_root(&self, path: &Path) -> Option<PathBuf> {
        let canonical_path = self.normalized_path(path);
        let mut best_root = None;
        let mut best_depth = 0usize;

        for entry in self.roots.iter() {
            let root = entry.key();
            let canonical_root = self.normalized_path(root);

            if !canonical_path.starts_with(&canonical_root) {
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

    /// Resolve an opened root by exact root identity.
    pub(super) fn configured_root(&self, root: &Path) -> Option<PathBuf> {
        if self.roots.contains_key(root) {
            return Some(root.to_path_buf());
        }

        let canonical_root = self.normalized_path(root);
        self.roots
            .iter()
            .find(|entry| self.normalized_path(entry.key()) == canonical_root)
            .map(|entry| entry.key().clone())
    }

    /// Resolve the configured root that owns a path.
    pub fn root_at(&self, path: &Path) -> Result<PathBuf, Error> {
        self.resolve_configured_root(path)
            .ok_or_else(|| Error::PathNotInRoot {
                path: path.to_path_buf(),
            })
    }

    /// Resolve the root whose current revision tracks a path.
    pub(super) fn resolve_repository_root(&self, path: &Path) -> Result<Option<PathBuf>, Error> {
        let canonical_path = self.normalized_path(path);
        let mut best_root = None;
        let mut best_depth = 0usize;

        for entry in self.roots.iter() {
            let session = &entry.value().session;
            let revision = session.revision(session.head())?;
            let repository = session.repository();
            let file_id = session.file_id(path);
            let is_member = repository.file(revision, file_id)?.is_some()
                || (canonical_path != path && {
                    let file_id = session.file_id(&canonical_path);

                    repository.file(revision, file_id)?.is_some()
                });
            if !is_member {
                continue;
            }

            let canonical_root = self.normalized_path(session.root());
            let depth = canonical_root.components().count();
            if depth <= best_depth {
                continue;
            }

            best_depth = depth;
            best_root = Some(session.root().to_path_buf());
        }

        Ok(best_root)
    }

    /// Resolve the opened root that owns one path.
    pub(crate) fn path_root(&self, path: &Path) -> Result<Arc<WorkspaceRoot>, Error> {
        let root = match self.resolve_configured_root(path) {
            Some(root) => root,
            None => self
                .resolve_repository_root(path)?
                .ok_or_else(|| Error::PathNotInRoot {
                    path: path.to_path_buf(),
                })?,
        };

        self.workspace_root(&root)
    }

    /// Resolve the exact opened root that owns one source edit.
    pub(crate) fn edit_root(&self, edit: &Edit) -> Result<Arc<WorkspaceRoot>, Error> {
        match edit {
            Edit::SetText { path, .. }
            | Edit::EditText { path, .. }
            | Edit::SetBytes { path, .. }
            | Edit::Remove { path } => self.path_root(path),
            Edit::Move { from, to } => {
                let root = self.path_root(from)?;
                self.resolve_path(root.as_ref(), to)?;

                Ok(root)
            }
        }
    }

    /// Return the canonical path when available, otherwise the original path.
    pub(crate) fn normalized_path(&self, path: &Path) -> PathBuf {
        if let Ok(path) = self.repository.file_system().canonicalize(path) {
            return path;
        }

        if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name())
            && let Ok(parent) = self.repository.file_system().canonicalize(parent)
        {
            return parent.join(file_name);
        }

        normalize_path(path)
    }

    /// Open one root.
    pub fn open_root(&self, root: PathBuf) -> Result<(), Error> {
        if self.roots.contains_key(&root) {
            return Ok(());
        }

        let session = Arc::new(self.build_session(root.clone())?);
        let workspace_root = Arc::new(WorkspaceRoot {
            session: Arc::clone(&session),
            writes: Mutex::new(()),
            watch: Arc::new(Mutex::new(WatchState::default())),
            background_run: Mutex::new(None),
        });
        let _write = workspace_root.writes.lock();
        self.roots.insert(root.clone(), Arc::clone(&workspace_root));

        // synchronize the current source state for the new root ref
        let result = session.reload_from_fs(session.head());
        let Err(error) = result else {
            return Ok(());
        };

        // rollback a failed root open so workspace state stays consistent
        self.roots.remove(root.as_path());

        Err(Error::from(error))
    }

    /// Close one root.
    pub fn close_root(&self, root: &Path) -> Result<(), Error> {
        let Some(root) = self.configured_root(root) else {
            return Ok(());
        };
        let workspace_root = self.workspace_root(root.as_path())?;
        let _write = workspace_root.writes.lock();

        // remove editor state owned by this root
        self.remove_open_files_under(root.as_path());

        // terminate subscriptions before removing the root
        workspace_root.watch.lock().close();

        // remove the root after dependent state is gone
        let _session = self.roots.remove(root.as_path());

        Ok(())
    }

    /// Clear all cache entries for every root.
    pub fn clear_cache_all(&self) -> Result<(), Error> {
        let cache_dir = self.repository.cache_directory();
        if !cache_dir.exists() {
            return Ok(());
        }

        std::fs::remove_dir_all(&cache_dir).map_err(|error| Error::Io {
            path: cache_dir.clone(),
            source: error,
        })?;

        Ok(())
    }

    /// Return the session for an opened root.
    pub fn session(&self, root: &Path) -> Result<Arc<Session>, Error> {
        let workspace_root = self.workspace_root(root)?;

        Ok(Arc::clone(&workspace_root.session))
    }

    /// Return one exact opened root.
    pub(crate) fn workspace_root(&self, root: &Path) -> Result<Arc<WorkspaceRoot>, Error> {
        let root = root.to_path_buf();
        let workspace_root = self
            .roots
            .get(root.as_path())
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| Error::PathNotInRoot { path: root.clone() })?;

        if workspace_root.session.root() != root {
            return Err(Error::Internal {
                detail: format!(
                    "session root mismatch: expected {}, found {}",
                    root.display(),
                    workspace_root.session.root().display()
                ),
            });
        }

        Ok(workspace_root)
    }

    /// Resolve the current revision for the session that owns a path.
    pub fn revision_at(&self, path: &Path) -> Result<Revision, Error> {
        let root = self.root_at(path)?;
        self.revision(&root)
    }

    /// Resolve the current revision for a root.
    pub fn revision(&self, root: &Path) -> Result<Revision, Error> {
        let session = self.session(root)?;
        session.revision(session.head()).map_err(Error::from)
    }

    /// Build one session for a root.
    fn build_session(&self, root: PathBuf) -> Result<Session, Error> {
        let revision_ref = Ref::for_root(root.as_path());

        if self.repository.current(&revision_ref).is_err() {
            let repository_root_ref = Ref::for_root(self.repository.path());
            self.repository
                .fork_ref(&repository_root_ref, revision_ref.clone())
                .map_err(Error::from)?;
        }

        let cwd = root.clone();

        Ok(Session::new(
            root,
            cwd,
            self.repository.clone(),
            revision_ref,
            self.worker_limit,
            self.event_handler.clone(),
        )?)
    }
}
