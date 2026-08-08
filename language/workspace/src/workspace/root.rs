use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_repository::{Ref, Repository, RepositoryError, Revision};
use destack_session::{ArtifactPriority, ArtifactRun, Session};
use parking_lot::{Mutex, MutexGuard};

use crate::diagnostic::Error;
use crate::{Watch, WatchState};

use super::LocalWorkspace;

/// One opened source root.
pub(crate) struct WorkspaceRoot {
    /// Physical source root.
    pub(crate) path: PathBuf,
    /// Moving repository ref for this root.
    pub(crate) head: Ref,
    /// Serialized root lifecycle and mutation state.
    state: Mutex<RootState>,
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
            .field("path", &self.path)
            .field("head", &self.head)
            .field("state", &self.state)
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
    /// Lock this root for one mutation while it remains open.
    pub(crate) fn write(&self) -> Result<MutexGuard<'_, RootState>, Error> {
        let state = self.state.lock();
        if *state == RootState::Closed {
            return Err(Error::PathNotInRoot {
                path: self.path.clone(),
            });
        }

        Ok(state)
    }

    /// Close this root and retain exclusive mutation access.
    fn close(&self) -> MutexGuard<'_, RootState> {
        let mut state = self.state.lock();
        *state = RootState::Closed;

        state
    }

    /// Return the revision currently published by this root.
    pub(crate) fn revision(&self, repository: &Repository) -> Result<Revision, Error> {
        repository.current(&self.head).map_err(Error::from)
    }

    /// Open one semantic watch at the current root revision.
    pub(crate) fn watch(&self, repository: &Repository) -> Result<Watch, Error> {
        let _write = self.write()?;
        let revision = self.revision(repository)?;

        Watch::new(self.path.clone(), revision, self.watch.clone())
    }

    /// Schedule proactive editor artifacts for one revision.
    pub(crate) fn schedule_background(
        &self,
        revision: Revision,
        artifacts: &[ArtifactKey],
        session: &Session,
    ) -> Result<(), Error> {
        let _write = self.write()?;
        let run = if artifacts.is_empty() {
            None
        } else {
            Some(session.provide(revision, artifacts, ArtifactPriority::Background))
        };
        let previous = std::mem::replace(&mut *self.background_run.lock(), run);

        // cancel obsolete work after publishing its replacement
        drop(previous);

        Ok(())
    }
}

/// Lifecycle state for one workspace root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootState {
    /// The root accepts operations.
    Open,
    /// The root no longer accepts operations.
    Closed,
}

impl LocalWorkspace {
    /// Terminate semantic subscriptions after one host watch failure.
    pub fn fail_watch(&self, root: &Path, detail: String) -> Result<(), Error> {
        let root = self.root(root)?;
        let _write = root.write()?;
        root.watch.lock().fail(detail);

        Ok(())
    }

    /// Open one root and return whether it was newly opened.
    pub fn open_root(&self, root: PathBuf) -> Result<bool, Error> {
        let root = self.normalized_path(&root);
        let mut roots = self.roots.write();
        if roots.contains_key(&root) {
            return Ok(false);
        }

        let head = self.open_head(&root)?;
        let workspace_root = Arc::new(WorkspaceRoot {
            path: root.clone(),
            head,
            state: Mutex::new(RootState::Open),
            watch: Arc::new(Mutex::new(WatchState::default())),
            background_run: Mutex::new(None),
        });

        // import source before exposing the opened root
        self.reload_locked(workspace_root.as_ref())?;
        roots.insert(root, workspace_root);

        Ok(true)
    }

    /// Close one root.
    pub fn close_root(&self, root: &Path) -> Result<(), Error> {
        let root = self.normalized_path(root);
        let workspace_root = self.roots.read().get(&root).cloned();
        let Some(workspace_root) = workspace_root else {
            return Ok(());
        };
        let _write = workspace_root.close();
        let mut roots = self.roots.write();

        // ignore a concurrent close that already removed this exact root
        let is_current = roots
            .get(&root)
            .is_some_and(|current| Arc::ptr_eq(current, &workspace_root));
        if !is_current {
            return Ok(());
        }

        // remove editor state owned by this root
        self.remove_open_files_under(&root);

        // terminate background work and subscriptions before removing the root
        let background_run = workspace_root.background_run.lock().take();
        drop(background_run);
        workspace_root.watch.lock().close();

        // remove the root after dependent state is gone
        roots.remove(root.as_path());

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

    /// Return the shared artifact computation session.
    pub fn session(&self) -> Arc<Session> {
        self.session.clone()
    }

    /// Return the current revision for the root that owns one path.
    pub fn revision_at(&self, path: &Path) -> Result<Revision, Error> {
        let root = self.root_at(path)?;
        self.revision(&root)
    }

    /// Resolve the current revision for a root.
    pub fn revision(&self, root: &Path) -> Result<Revision, Error> {
        self.root(root)?.revision(&self.repository)
    }

    /// Open the moving repository ref for one root.
    fn open_head(&self, root: &Path) -> Result<Ref, Error> {
        let head = Ref::for_root(root);

        match self.repository.current(&head) {
            Ok(_revision) => {}
            Err(RepositoryError::MissingRef { .. }) => {
                let origin = Ref::for_root(self.repository.path());
                self.repository
                    .fork_ref(&origin, head.clone())
                    .map_err(Error::from)?;
            }
            Err(error) => return Err(Error::from(error)),
        }

        Ok(head)
    }
}
