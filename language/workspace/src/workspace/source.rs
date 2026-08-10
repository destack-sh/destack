use std::path::Path;
use std::sync::Arc;

use destack_repository as repository;
use destack_repository::{Commit, RepositoryError, Revision, RevisionPin};
use destack_source::{
    Edit, File, FileId, FilePatch, ModuleId, Patch, Span, TextPatch, apply_file_patch,
};

use crate::Error;
use crate::file::normalize_path;

use super::Workspace;

impl Workspace {
    /// Return one workspace-relative logical path.
    pub(crate) fn logical_path(&self, path: &Path) -> Result<String, Error> {
        let path = if let Ok(path) = self.repository.file_system().canonicalize(path) {
            path
        } else if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name())
            && let Ok(parent) = self.repository.file_system().canonicalize(parent)
        {
            parent.join(file_name)
        } else {
            normalize_path(path)
        };
        let logical_path = path
            .strip_prefix(&self.root)
            .map_err(|_| Error::PathNotInRoot { path: path.clone() })?;

        Ok(Self::path_text(logical_path))
    }

    /// Return one workspace-relative file id.
    pub(crate) fn file_id(&self, path: &Path) -> Result<FileId, Error> {
        let path = self.logical_path(path)?;

        Ok(FileId::from_logical_str(&path))
    }

    /// Return the shared file for one path in one revision.
    pub(crate) fn file(&self, revision: Revision, path: &Path) -> Result<Arc<File>, Error> {
        let file_id = self.file_id(path)?;
        let file = self
            .repository
            .file(revision, file_id)?
            .ok_or_else(|| Error::Internal {
                detail: format!("file is missing from revision: {}", path.display()),
            })?;

        Ok(file)
    }

    /// Load one filesystem module into a private revision.
    pub(crate) fn load_module(
        &self,
        revision: RevisionPin,
        path: &Path,
    ) -> Result<(RevisionPin, ModuleId), Error> {
        let before = revision.revision();
        let logical_path = self.logical_path(path)?;
        let logical_path = Path::new(&logical_path);

        // reuse a module already tracked by this revision
        if let Some(module_id) = self.repository.module_id_for_path(before, logical_path)? {
            return Ok((revision, module_id));
        }

        // load the requested physical source file
        let edits = self
            .repository
            .scan_file(&self.root, before, logical_path.to_path_buf())?;
        let after = self.repository.edit(before, edits)?.after;
        let revision = self.repository.pin(after)?;

        // require the imported file to produce a module
        let module_id = self.repository.module_id_for_path(after, logical_path)?;
        let Some(module_id) = module_id else {
            return Err(Error::ModuleNotLoadable {
                path: path.to_path_buf(),
                detail: "loaded source file did not produce a module".to_string(),
            });
        };

        Ok((revision, module_id))
    }

    /// Commit source edits against this workspace's exact revision.
    pub(crate) fn commit(&self, revision: Revision, edits: Vec<Edit>) -> Result<Commit, Error> {
        let current = self.revision()?;
        if current != revision {
            return Err(Error::StaleRevision {
                expected: revision,
                current,
            });
        }

        // lower every source edit against the same immutable revision
        let edits = edits
            .into_iter()
            .map(|edit| self.lower(revision, edit))
            .collect::<Result<Vec<_>, _>>()?;

        self.advance(revision, edits)
    }

    /// Advance this workspace with repository edits.
    pub(crate) fn advance(
        &self,
        revision: Revision,
        edits: Vec<repository::Edit>,
    ) -> Result<Commit, Error> {
        self.repository
            .commit(&self.head, revision, edits)
            .map_err(|error| match error {
                RepositoryError::RefChanged {
                    expected, current, ..
                } => Error::StaleRevision { expected, current },
                error => Error::from(error),
            })
    }

    /// Lower one source edit into one repository edit.
    pub(crate) fn lower(&self, revision: Revision, edit: Edit) -> Result<repository::Edit, Error> {
        let edit = match edit {
            Edit::SetText { path, text } => {
                let path = self.logical_path(&path)?;
                let blob = self.repository.put_blob(text.as_bytes())?;

                repository::Edit::set_file(path, blob)
            }
            Edit::EditText { path, patches } => {
                let logical_path = self.logical_path(&path)?;
                let text = self.apply_text_patches(revision, &path, &logical_path, patches)?;
                let blob = self.repository.put_blob(text.as_bytes())?;

                repository::Edit::set_file(logical_path, blob)
            }
            Edit::SetBytes { path, bytes } => {
                let logical_path = self.logical_path(&path)?;
                let blob = self.repository.put_blob(&bytes)?;

                repository::Edit::set_file(logical_path, blob)
            }
            Edit::Remove { path } => {
                let path = self.logical_path(&path)?;

                repository::Edit::remove_file(path)
            }
            Edit::Move { from, to } => {
                let from = self.logical_path(&from)?;
                let to = self.logical_path(&to)?;

                repository::Edit::move_file(from, to)
            }
        };

        Ok(edit)
    }

    /// Apply text patches to one tracked file.
    fn apply_text_patches(
        &self,
        revision: Revision,
        path: &Path,
        logical_path: &str,
        patches: Vec<TextPatch>,
    ) -> Result<String, Error> {
        let file_id = FileId::from_logical_str(logical_path);
        let file = self
            .repository
            .file(revision, file_id)?
            .ok_or_else(|| Error::FileMissing {
                path: path.to_path_buf(),
            })?;

        // lower protocol ranges into source patches
        let patches = patches
            .into_iter()
            .map(|patch| {
                Patch::replace(
                    Span::new(file_id, patch.range.start, patch.range.end),
                    patch.text,
                )
            })
            .collect();
        let patch = FilePatch::with_patches(file_id, patches);

        apply_file_patch(file.as_ref(), &patch).map_err(|error| Error::InvalidTextChange {
            path: path.to_path_buf(),
            detail: error.to_string(),
        })
    }

    /// Return one path as normalized logical text.
    fn path_text(path: impl AsRef<Path>) -> String {
        path.as_ref().to_string_lossy().replace('\\', "/")
    }
}
