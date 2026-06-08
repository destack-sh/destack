use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{Ref, Revision};
use destack_session::Session;

use crate::diagnostic::Error;

use super::Workspace;

impl Workspace {
    /// Resolve the configured root that owns a path.
    pub(super) fn owned_root(&self, path: &Path) -> Option<PathBuf> {
        let canonical_path = Self::normalized_path(path);
        let mut best_root = None;
        let mut best_depth = 0usize;

        for entry in self.roots.iter() {
            let root = entry.key();
            let canonical_root = Self::normalized_path(root);

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

    /// Resolve the configured root that owns a path.
    pub fn root_at(&self, path: &Path) -> Result<PathBuf, Error> {
        self.owned_root(path).ok_or_else(|| Error::PathNotInRoot {
            path: path.to_path_buf(),
        })
    }

    /// Resolve the root whose current revision tracks a path.
    pub(super) fn tracked_root(&self, path: &Path) -> Result<Option<PathBuf>, Error> {
        let canonical_path = Self::normalized_path(path);
        let mut best_root = None;
        let mut best_depth = 0usize;

        for entry in self.roots.iter() {
            let session = entry.value();
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

            let canonical_root = Self::normalized_path(session.root());
            let depth = canonical_root.components().count();
            if depth <= best_depth {
                continue;
            }

            best_depth = depth;
            best_root = Some(session.root().to_path_buf());
        }

        Ok(best_root)
    }

    /// Resolve the session that should own one file edit.
    pub(crate) fn edit_session(&self, path: &Path) -> Result<Arc<Session>, Error> {
        // prefer configured root ownership for local edits
        if let Some(root) = self.owned_root(path) {
            return self.session(&root);
        }

        // fall back to tracked repository ownership
        self.session_at(path)
    }

    /// Return the canonical path when available, otherwise the original path.
    pub(crate) fn normalized_path(path: &Path) -> PathBuf {
        match std::fs::canonicalize(path) {
            Ok(path) => path,
            Err(_error) => path.to_path_buf(),
        }
    }

    /// Open one root.
    pub fn open_root(&self, root: PathBuf) -> Result<(), Error> {
        if self.roots.contains_key(&root) {
            return Ok(());
        }

        let session = Arc::new(self.build_session(root.clone())?);
        self.roots.insert(root.clone(), Arc::clone(&session));

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
        let root = root.to_path_buf();
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

    /// Return the number of active roots.
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Resolve or create the session for a root.
    pub(crate) fn session(&self, root: &Path) -> Result<Arc<Session>, Error> {
        let root = root.to_path_buf();

        if !self.roots.contains_key(&root) {
            self.open_root(root.clone())?;
        }

        let session = self
            .roots
            .get(root.as_path())
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| Error::PathNotInRoot { path: root.clone() })?;

        if session.root() != root {
            return Err(Error::Internal {
                detail: format!(
                    "session root mismatch: expected {}, found {}",
                    root.display(),
                    session.root().display()
                ),
            });
        }

        Ok(session)
    }

    /// Resolve or create the session that owns a path.
    pub(super) fn session_at(&self, path: &Path) -> Result<Arc<Session>, Error> {
        let Some(root) = self.tracked_root(path)? else {
            return Err(Error::PathNotInRoot {
                path: path.to_path_buf(),
            });
        };

        self.session(&root)
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
            self.compiler.clone(),
            self.linter.clone(),
            self.query.clone(),
            self.worker_limit,
            self.event_handler.clone(),
        )?)
    }
}
