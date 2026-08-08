use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_repository::{FileSystemSource, Revision};
use destack_source::{Edit, TextChange, Uri, apply_text_changes};

use crate::diagnostic::Error;
use crate::file::Commit;
use crate::workspace::{LocalWorkspace, WorkspaceRoot};

impl LocalWorkspace {
    /// Open one file with its current content.
    pub fn open_file(&self, uri: Uri, version: i32, edit: Edit) -> Result<Commit, Error> {
        self.change_file(uri, version, edit)
    }

    /// Change one open file to its current content.
    pub fn change_file(&self, uri: Uri, version: i32, edit: Edit) -> Result<Commit, Error> {
        let path = match &edit {
            Edit::SetText { path, .. } | Edit::SetBytes { path, .. } | Edit::Remove { path } => {
                path.clone()
            }
            Edit::EditText { .. } => {
                return Err(Error::InvalidEdit {
                    detail: "text patch edits require patch_text_file".to_string(),
                });
            }
            Edit::Move { .. } => {
                return Err(Error::InvalidEdit {
                    detail: "move edits cannot represent open file content".to_string(),
                });
            }
        };
        let root = self.owner(path.as_path())?;
        let _write = root.write()?;

        self.commit_open_file(root.as_ref(), path, uri, version, edit)
    }

    /// Commit one open file change while its root is locked.
    fn commit_open_file(
        &self,
        root: &WorkspaceRoot,
        path: PathBuf,
        uri: Uri,
        version: i32,
        edit: Edit,
    ) -> Result<Commit, Error> {
        if let Some(current) = self.open_file_version(path.as_path())
            && version <= current
        {
            return Err(Error::StaleOpenFile {
                path,
                incoming: version,
                current,
            });
        }

        // advance the root to the open content
        let is_removed = edit.is_remove();
        let open_content = edit.clone();
        let revision = root.revision(&self.repository)?;
        let pending = root.prepare(revision, vec![edit], self)?;

        // removed files are no longer open source truth
        if is_removed {
            let commit = pending.advance(root, self)?;
            self.remove_open_state(path.as_path());

            return Ok(commit.publish(root));
        }

        // resolve client identity before advancing the root
        let content_id = self.content_id(root, pending.after(), path.as_path())?;
        let file_id = root.file_id(&self.repository, &path)?;
        let mut commit = pending.advance(root, self)?;

        // publish editor state before notifying semantic watches
        self.set_open_state(
            path.as_path(),
            uri.clone(),
            version,
            content_id,
            open_content,
        );
        if let Some(update) = commit
            .updates
            .iter_mut()
            .find(|update| update.file_id == file_id)
        {
            update.diagnostic_uri = uri;
            update.diagnostic_version = Some(version);
        }

        Ok(commit.publish(root))
    }

    /// Patch one open text file.
    pub fn patch_text_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        changes: Vec<TextChange>,
    ) -> Result<Commit, Error> {
        let root = self.owner(path)?;
        let _write = root.write()?;

        // reject stale client versions before computing text
        if let Some(current) = self.open_file_version(path)
            && version <= current
        {
            return Err(Error::StaleOpenFile {
                path: path.to_path_buf(),
                incoming: version,
                current,
            });
        }

        // apply the patch to the current open text
        let Some(text) = self.open_file_text(path) else {
            return Err(Error::InvalidTextChange {
                path: path.to_path_buf(),
                detail: "open file text is not available".to_string(),
            });
        };
        let content =
            apply_text_changes(text, &changes).map_err(|error| Error::InvalidTextChange {
                path: path.to_path_buf(),
                detail: error.to_string(),
            })?;

        let edit = Edit::SetText {
            path: path.to_path_buf(),
            text: content,
        };

        self.commit_open_file(root.as_ref(), path.to_path_buf(), uri, version, edit)
    }

    /// Save one open file to explicit content.
    pub fn save_file(&self, edit: Edit) -> Result<Commit, Error> {
        let path = match &edit {
            Edit::SetText { path, .. } | Edit::SetBytes { path, .. } | Edit::Remove { path } => {
                path.clone()
            }
            Edit::EditText { .. } => {
                return Err(Error::InvalidEdit {
                    detail: "text patch edits require patch_text_file".to_string(),
                });
            }
            Edit::Move { .. } => {
                return Err(Error::InvalidEdit {
                    detail: "move edits cannot represent saved file content".to_string(),
                });
            }
        };
        let root = self.owner(path.as_path())?;
        let _write = root.write()?;

        // advance the root to the saved content
        let file = self.open_state(path.as_path());
        let is_removed = edit.is_remove();
        let open_content = edit.clone();
        let revision = root.revision(&self.repository)?;
        let pending = root.prepare(revision, vec![edit], self)?;

        // remove open state after publishing file removal
        if let Some(file) = file {
            if is_removed {
                let commit = pending.advance(root.as_ref(), self)?;
                self.remove_open_state(path.as_path());

                return Ok(commit.publish(root.as_ref()));
            }

            // resolve client identity before advancing the root
            let content_id = self.content_id(root.as_ref(), pending.after(), path.as_path())?;
            let file_id = root.file_id(&self.repository, &path)?;
            let mut commit = pending.advance(root.as_ref(), self)?;

            // publish editor state before notifying semantic watches
            self.set_open_state(
                path.as_path(),
                file.uri.clone(),
                file.version,
                content_id,
                open_content,
            );
            if let Some(update) = commit
                .updates
                .iter_mut()
                .find(|update| update.file_id == file_id)
            {
                update.diagnostic_uri = file.uri;
                update.diagnostic_version = Some(file.version);
            }

            return Ok(commit.publish(root.as_ref()));
        }

        let commit = pending.advance(root.as_ref(), self)?;

        Ok(commit.publish(root.as_ref()))
    }

    /// Save text content from explicit content or host filesystem.
    pub fn save_text_file(&self, path: &Path, content: Option<String>) -> Result<Commit, Error> {
        let content = match content {
            Some(content) => content,
            None => self
                .repository
                .file_system()
                .read_to_string(path)
                .map_err(|source| Error::Io {
                    path: path.to_path_buf(),
                    source,
                })?,
        };

        self.save_file(Edit::SetText {
            path: path.to_path_buf(),
            text: content,
        })
    }

    /// Save binary content from explicit content or host filesystem.
    pub fn save_bytes_file(&self, path: &Path, content: Option<Vec<u8>>) -> Result<Commit, Error> {
        let content = match content {
            Some(content) => content,
            None => self
                .repository
                .file_system()
                .read(path)
                .map_err(|source| Error::Io {
                    path: path.to_path_buf(),
                    source,
                })?,
        };

        self.save_file(Edit::SetBytes {
            path: path.to_path_buf(),
            bytes: content,
        })
    }

    /// Close one open file and restore filesystem backed source truth.
    pub fn close_file(&self, path: &Path) -> Result<Option<Commit>, Error> {
        let root = self.owner(path)?;
        let _write = root.write()?;

        // remove overlay state first so filesystem reads see disk truth
        let Some(file) = self.remove_open_state(path) else {
            return Ok(None);
        };

        // read the current filesystem truth for this path
        let edit = match FileSystemSource::read_edit(self.repository.as_ref(), path) {
            Ok(edit) => edit,
            Err(error) if error.kind() == ErrorKind::NotFound => Edit::Remove {
                path: path.to_path_buf(),
            },
            Err(error) => {
                self.set_open_state(path, file.uri, file.version, file.content_id, file.content);

                return Err(Error::Io {
                    path: path.to_path_buf(),
                    source: error,
                });
            }
        };

        // restore the open overlay if the repository update fails
        let revision = root.revision(&self.repository)?;
        let commit = match self.commit(root.as_ref(), revision, vec![edit]) {
            Ok(commit) => commit,
            Err(error) => {
                self.set_open_state(path, file.uri, file.version, file.content_id, file.content);

                return Err(error);
            }
        };

        Ok(Some(commit))
    }

    /// Apply one edit through the workspace.
    pub fn apply_file(&self, edit: Edit) -> Result<Commit, Error> {
        let root = self.edit_root(&edit)?;
        let _write = root.write()?;
        let edit = self.resolve_edit(root.as_ref(), edit)?;

        // apply direct edits through the owning root
        let revision = root.revision(&self.repository)?;
        let commit = self.commit(root.as_ref(), revision, vec![edit])?;

        Ok(commit)
    }

    /// Apply atomic edits through the workspace.
    pub fn apply_source_edits(&self, root: &Path, edits: Vec<Edit>) -> Result<Commit, Error> {
        let root = self.root(root)?;
        let _write = root.write()?;
        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(root.as_ref(), edit))
            .collect::<Result<Vec<_>, _>>()?;

        // publish the edit batch through the owning root
        let revision = root.revision(&self.repository)?;
        let commit = self.commit(root.as_ref(), revision, edits)?;

        Ok(commit)
    }

    /// Apply atomic edits when the current revision still matches.
    pub fn apply_source_edits_if_current(
        &self,
        root: &Path,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, Error> {
        let root = self.root(root)?;
        let _write = root.write()?;
        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(root.as_ref(), edit))
            .collect::<Result<Vec<_>, _>>()?;

        // publish the edit batch through the owning root
        let commit = self.commit(root.as_ref(), revision, edits)?;

        Ok(commit)
    }
}
