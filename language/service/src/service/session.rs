use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::Compiler;
use destack_session::Session;
use destack_workspace::{Ref, Repository, Revision};

use super::{LanguageService, LanguageServiceError};

impl LanguageService {
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
    pub fn root_at(&self, path: &Path) -> Result<PathBuf, LanguageServiceError> {
        self.owned_root(path)
            .ok_or_else(|| LanguageServiceError::PathNotInRoot {
                path: path.to_path_buf(),
            })
    }

    /// Resolve the root whose current revision tracks a path.
    pub(super) fn tracked_root(
        &self,
        path: &Path,
    ) -> Result<Option<PathBuf>, LanguageServiceError> {
        let canonical_path = Self::normalized_path(path);
        let mut best_root = None;
        let mut best_depth = 0usize;

        for entry in self.roots.iter() {
            let session = entry.value();
            let revision = session.revision(session.head())?;
            let repository = session.repository();
            let file_id = repository.file_id(path);
            let is_member = repository.file(revision, file_id)?.is_some()
                || (canonical_path != path && {
                    let file_id = repository.file_id(&canonical_path);

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
    pub(super) fn edit_session(&self, path: &Path) -> Result<Arc<Session>, LanguageServiceError> {
        // prefer configured root ownership for local edits
        if let Some(root) = self.owned_root(path) {
            return self.session(&root);
        }

        // fall back to tracked repository ownership
        self.session_at(path)
    }

    /// Return the canonical path when available, otherwise the original path.
    pub(super) fn normalized_path(path: &Path) -> PathBuf {
        match std::fs::canonicalize(path) {
            Ok(path) => path,
            Err(_error) => path.to_path_buf(),
        }
    }

    /// Open one root.
    pub fn open_root(&self, root: PathBuf) -> Result<(), LanguageServiceError> {
        if self.roots.contains_key(&root) {
            return Ok(());
        }

        let session = Arc::new(self.build_session(root.clone())?);
        self.roots.insert(root.clone(), Arc::clone(&session));

        // synchronize the current source state for the new workspace ref
        let result = session.reload_from_fs(session.head());
        let Err(error) = result else {
            return Ok(());
        };

        // rollback a failed root open so service state stays consistent
        self.roots.remove(root.as_path());

        Err(LanguageServiceError::from(error))
    }

    /// Close one root.
    pub fn close_root(&self, root: &Path) -> Result<(), LanguageServiceError> {
        let root = root.to_path_buf();
        let _session = self.roots.remove(root.as_path());

        Ok(())
    }

    /// Clear all cache entries for every root.
    pub fn clear_cache_all(&self) -> Result<(), LanguageServiceError> {
        let cache_dir = self.repository.cache_directory();
        if !cache_dir.exists() {
            return Ok(());
        }

        std::fs::remove_dir_all(&cache_dir).map_err(|error| LanguageServiceError::Io {
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
    pub(super) fn session(&self, root: &Path) -> Result<Arc<Session>, LanguageServiceError> {
        let root = root.to_path_buf();

        if !self.roots.contains_key(&root) {
            self.open_root(root.clone())?;
        }

        let session = self
            .roots
            .get(root.as_path())
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| LanguageServiceError::PathNotInRoot { path: root.clone() })?;

        if session.root() != root {
            return Err(LanguageServiceError::Internal {
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
    pub(super) fn session_at(&self, path: &Path) -> Result<Arc<Session>, LanguageServiceError> {
        let Some(root) = self.tracked_root(path)? else {
            return Err(LanguageServiceError::PathNotInRoot {
                path: path.to_path_buf(),
            });
        };

        self.session(&root)
    }

    /// Resolve the current revision for the session that owns a path.
    pub fn revision_at(&self, path: &Path) -> Result<Revision, LanguageServiceError> {
        let root = self.root_at(path)?;
        self.revision(&root)
    }

    /// Resolve the current revision for a root.
    pub fn revision(&self, root: &Path) -> Result<Revision, LanguageServiceError> {
        let session = self.session(root)?;
        session
            .revision(session.head())
            .map_err(LanguageServiceError::from)
    }

    /// Return repository and compiler handles for one root.
    pub fn root_handles(
        &self,
        root: &Path,
    ) -> Result<(Arc<Repository>, Arc<Compiler>), LanguageServiceError> {
        let session = self.session(root)?;

        Ok((session.repository(), session.compiler()))
    }

    /// Build one session for a root.
    fn build_session(&self, root: PathBuf) -> Result<Session, LanguageServiceError> {
        let revision_ref = Ref::for_workspace_root(root.as_path());

        if self.repository.current(&revision_ref).is_err() {
            let repository_root_ref = Ref::for_workspace_root(self.repository.workspace_root());
            self.repository
                .fork_ref(&repository_root_ref, revision_ref.clone())
                .map_err(LanguageServiceError::from)?;
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
