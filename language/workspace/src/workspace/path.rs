use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::Edit;

use crate::diagnostic::Error;
use crate::file::{FileOperation, normalize_path};

use super::{LocalWorkspace, WorkspaceRoot};

impl LocalWorkspace {
    /// Return one exact opened root.
    pub(crate) fn root(&self, root: &Path) -> Result<Arc<WorkspaceRoot>, Error> {
        let normalized = self.normalized_path(root);
        let workspace_root = self.roots.read().get(&normalized).cloned();

        workspace_root.ok_or_else(|| Error::PathNotInRoot {
            path: root.to_path_buf(),
        })
    }

    /// Return the opened root that owns one path.
    pub(crate) fn owner(&self, path: &Path) -> Result<Arc<WorkspaceRoot>, Error> {
        let mut roots = self.roots_at(path);
        let owner = roots.pop().ok_or_else(|| Error::PathNotInRoot {
            path: path.to_path_buf(),
        })?;

        self.root(&owner)
    }

    /// Return every opened root containing one physical path, shallowest first.
    pub fn roots_at(&self, path: &Path) -> Vec<PathBuf> {
        let path = self.normalized_path(path);
        let roots = self.roots.read();
        let mut roots = roots
            .values()
            .filter(|root| path.starts_with(&root.path))
            .map(|root| root.path.clone())
            .collect::<Vec<_>>();
        roots.sort_by_key(|root| root.components().count());

        roots
    }

    /// Return the opened root path that owns one path.
    pub fn root_at(&self, path: &Path) -> Result<PathBuf, Error> {
        let root = self.owner(path)?;

        Ok(root.path.clone())
    }

    /// Resolve one source path within one exact opened root.
    pub(crate) fn resolve_path(&self, root: &WorkspaceRoot, path: &Path) -> Result<PathBuf, Error> {
        // anchor relative paths at the requested root
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.path.join(path)
        };

        // require the path to belong to the requested root
        let owner = self.owner(&path)?;
        if owner.path != root.path {
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

    /// Return the opened root that owns one source edit.
    pub(crate) fn edit_root(&self, edit: &Edit) -> Result<Arc<WorkspaceRoot>, Error> {
        match edit {
            Edit::SetText { path, .. }
            | Edit::EditText { path, .. }
            | Edit::SetBytes { path, .. }
            | Edit::Remove { path } => self.owner(path),
            Edit::Move { from, to } => {
                let root = self.owner(from)?;
                self.resolve_path(root.as_ref(), to)?;

                Ok(root)
            }
        }
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
