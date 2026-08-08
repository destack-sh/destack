use std::path::Path;
use std::sync::Arc;

use destack_repository as repository;
use destack_repository::{FileSystemSource, Repository, Revision, RevisionPin};
use destack_source::{
    Content, Edit, FileId, FilePatch, ModuleId, Patch, Span, TextPatch, apply_file_patch,
};

use crate::Error;
use crate::file::normalize_path;

use super::WorkspaceRoot;

impl WorkspaceRoot {
    /// Return one root relative logical path.
    pub(crate) fn logical_path(
        &self,
        repository: &Repository,
        path: &Path,
    ) -> Result<String, Error> {
        let path = if let Ok(path) = repository.file_system().canonicalize(path) {
            path
        } else if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name())
            && let Ok(parent) = repository.file_system().canonicalize(parent)
        {
            parent.join(file_name)
        } else {
            normalize_path(path)
        };
        let logical_path = path
            .strip_prefix(&self.path)
            .map_err(|_| Error::PathNotInRoot { path: path.clone() })?;

        Ok(Self::path_text(logical_path))
    }

    /// Return one root relative file id.
    pub(crate) fn file_id(&self, repository: &Repository, path: &Path) -> Result<FileId, Error> {
        let path = self.logical_path(repository, path)?;

        Ok(FileId::from_logical_str(&path))
    }

    /// Load one filesystem module into a private revision.
    pub(crate) fn load_module(
        &self,
        repository: &Arc<Repository>,
        revision: RevisionPin,
        path: &Path,
    ) -> Result<(RevisionPin, ModuleId), Error> {
        let before = revision.revision();
        let logical_path = self.logical_path(repository, path)?;
        let logical_path = Path::new(&logical_path);

        // reuse a module already tracked by this revision
        if let Some(module_id) = repository.module_id_for_path(before, logical_path)? {
            return Ok((revision, module_id));
        }

        // import the requested physical source file
        let source = FileSystemSource::new(repository.as_ref(), &self.path, before);
        let Some(edits) = source.edits_for_path(logical_path)? else {
            return Err(Error::ModuleNotLoadable {
                path: path.to_path_buf(),
                detail: "source file is not loadable".to_string(),
            });
        };
        let after = repository.commit_edits(before, edits)?;
        let revision = repository.pin(after)?;

        // require the imported file to produce a module
        let module_id = repository.module_id_for_path(after, logical_path)?;
        let Some(module_id) = module_id else {
            return Err(Error::ModuleNotLoadable {
                path: path.to_path_buf(),
                detail: "loaded source file did not produce a module".to_string(),
            });
        };

        Ok((revision, module_id))
    }

    /// Lower one source edit into one repository edit.
    pub(crate) fn lower(
        &self,
        repository: &Repository,
        revision: Revision,
        edit: Edit,
    ) -> Result<repository::Edit, Error> {
        let edit = match edit {
            Edit::SetText { path, text } => {
                let path = self.logical_path(repository, &path)?;

                repository::Edit::set_text(path, text)
            }
            Edit::EditText { path, patches } => {
                let logical_path = self.logical_path(repository, &path)?;
                let text =
                    self.apply_text_patches(repository, revision, &path, &logical_path, patches)?;

                repository::Edit::set_text(logical_path, text)
            }
            Edit::SetBytes { path, bytes } => {
                let logical_path = self.logical_path(repository, &path)?;

                repository::Edit::SetFile {
                    logical_path,
                    content: Content::Binary { content: bytes },
                }
            }
            Edit::Remove { path } => {
                let path = self.logical_path(repository, &path)?;

                repository::Edit::remove_file(path)
            }
            Edit::Move { from, to } => {
                let from = self.logical_path(repository, &from)?;
                let to = self.logical_path(repository, &to)?;

                repository::Edit::move_file(from, to)
            }
        };

        Ok(edit)
    }

    /// Apply text patches to one tracked file.
    fn apply_text_patches(
        &self,
        repository: &Repository,
        revision: Revision,
        path: &Path,
        logical_path: &str,
        patches: Vec<TextPatch>,
    ) -> Result<String, Error> {
        let file_id = FileId::from_logical_str(logical_path);
        let file = repository
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
