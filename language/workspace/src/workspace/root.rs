use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{Ref, Revision};
use destack_serde::Reflect;
use destack_session::Session;
use serde::{Deserialize, Serialize};

use crate::diagnostic::Error;
use crate::file::normalize_path;

use super::LocalWorkspace;

/// Request to reload host source state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReloadRequest {
    /// Roots to reload.
    pub roots: Vec<PathBuf>,
    /// Reason for the reload.
    pub reason: ReloadReason,
}

/// Reason for reloading host source state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ReloadReason {
    /// Reload requested by the caller.
    Manual,
    /// Reload required after a watcher overflow.
    Overflow,
    /// Reload required after watch roots changed.
    Watch,
}

impl LocalWorkspace {
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

    /// Resolve the session that should own one edit.
    pub(crate) fn edit_session(&self, path: &Path) -> Result<Arc<Session>, Error> {
        // prefer configured root ownership for local edits
        if let Some(root) = self.resolve_configured_root(path) {
            return self.session(&root);
        }

        // fall back to tracked repository ownership
        self.session_at(path)
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
        let Some(root) = self.configured_root(root) else {
            return Ok(());
        };

        self.remove_open_files_under(root.as_path());
        if let Some((_, watch)) = self.watches.remove(root.as_path()) {
            watch.stop();
        }
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

    /// Resolve or create the session for a root.
    pub fn session(&self, root: &Path) -> Result<Arc<Session>, Error> {
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
        let Some(root) = self.resolve_repository_root(path)? else {
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
