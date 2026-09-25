use std::path::{Path, PathBuf};
use std::sync::Arc;

use tspp_repository::{Package, Revision};
use tspp_source::Edit;

use crate::Error;
use crate::file::normalize_path;

use super::Workspace;

impl Workspace {
    /// Resolve this workspace's canonical root identity.
    pub(crate) fn resolve_root(&self, root: &Path) -> Result<&Path, Error> {
        let root = self.normalized_path(root);
        if root != self.root {
            return Err(Error::PathNotInRoot { path: root });
        }

        Ok(&self.root)
    }

    /// Return the nearest package owning one physical path.
    pub fn nearest_package(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<Package>>, Error> {
        let path = self.normalized_path(path);
        let Ok(path) = path.strip_prefix(&self.root) else {
            return Ok(None);
        };

        self.repository
            .nearest_package(revision, path)
            .map_err(Error::from)
    }

    /// Resolve one source path within this workspace.
    pub(crate) fn resolve_path(&self, path: &Path) -> Result<PathBuf, Error> {
        // anchor relative paths at the workspace root
        let path = if path.is_absolute() {
            self.normalized_path(path)
        } else {
            normalize_path(&self.root.join(path))
        };

        // require every resolved path to remain inside the workspace
        if !path.starts_with(&self.root) {
            return Err(Error::PathNotInRoot { path });
        }

        Ok(path)
    }

    /// Resolve every path in one source edit.
    pub(crate) fn resolve_edit(&self, mut edit: Edit) -> Result<Edit, Error> {
        match &mut edit {
            Edit::SetText { path, .. }
            | Edit::EditText { path, .. }
            | Edit::SetBytes { path, .. }
            | Edit::Remove { path } => *path = self.resolve_path(path)?,
            Edit::Move { from, to } => {
                *from = self.resolve_path(from)?;
                *to = self.resolve_path(to)?;
            }
        }

        Ok(edit)
    }

    /// Return the canonical path when available, otherwise the lexical path.
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
}
