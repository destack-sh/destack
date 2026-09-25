use std::path::Path;

use tspp_repository as repository;
use tspp_repository::{Change, Commit, Revision, RevisionPin, Trace};
use tspp_source::{Edit, FileId, FilePatch, ModuleId, Patch, Span, TextPatch, apply_file_patch};

use crate::Error;
use crate::file::normalize_path;

use super::Workspace;

impl Workspace {
    /// Compare two exact workspace revisions.
    pub fn diff(&self, before: Revision, after: Revision) -> Result<Vec<Change>, Error> {
        let _before = self.repository.pin(before)?;
        let _after = self.repository.pin(after)?;

        self.repository.changes(before, after).map_err(Error::from)
    }

    /// List files at one exact workspace revision.
    pub fn files(&self, revision: Revision) -> Result<Vec<repository::File>, Error> {
        let _revision = self.repository.pin(revision)?;

        self.repository.files(revision).map_err(Error::from)
    }

    /// Commit source edits to one exact branch revision.
    pub fn edit_branch(
        &self,
        name: &str,
        revision: Revision,
        edits: Vec<Edit>,
        trace: &Trace,
    ) -> Result<Commit, Error> {
        let mut state = trace.span("branch.lock", || self.lock())?;
        let current = state.branch(name)?.revision();
        if current != revision {
            return Err(Error::StaleRevision {
                expected: revision,
                current,
            });
        }

        trace.add_counter("edit.changes", edits.len() as u64);
        let edits = trace.span("edit.resolve", || {
            edits
                .into_iter()
                .map(|edit| self.resolve_edit(edit))
                .collect::<Result<Vec<_>, _>>()
        })?;
        let edits = trace.span("edit.lower", || {
            edits
                .into_iter()
                .map(|edit| self.lower(revision, edit))
                .collect::<Result<Vec<_>, _>>()
        })?;
        let commit = trace.span("revision.edit", || self.repository.edit(revision, edits))?;
        let after = trace.span("revision.pin", || self.repository.pin(commit.after))?;
        trace.span("branch.publish", || {
            state.branches.insert(name.to_string(), after.clone());
            self.publish(Some(name), &commit, after);
        });

        Ok(commit)
    }

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

    /// Lower one source edit into one repository edit.
    pub(crate) fn lower(&self, revision: Revision, edit: Edit) -> Result<repository::Edit, Error> {
        let edit = match edit {
            Edit::SetText { path, text } => {
                let path = self.logical_path(&path)?;
                let blob = self.repository.retain_blob(text.as_bytes())?;

                repository::Edit::set_file(path, blob)
            }
            Edit::EditText { path, patches } => {
                let logical_path = self.logical_path(&path)?;
                let text = self.apply_text_patches(revision, &path, &logical_path, patches)?;
                let blob = self.repository.retain_blob(text.as_bytes())?;

                repository::Edit::set_file(logical_path, blob)
            }
            Edit::SetBytes { path, bytes } => {
                let logical_path = self.logical_path(&path)?;
                let blob = self.repository.retain_blob(&bytes)?;

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
