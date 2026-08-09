use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_repository::{Commit, Revision};
use destack_source::{Content, Edit, FileType, TextChange, Uri, apply_text_changes};

use crate::Error;
use crate::file::{FileOperation, OpenFile};
use crate::workspace::Workspace;

impl Workspace {
    /// Open one file with its current content.
    pub fn open_file(&self, uri: Uri, version: i32, edit: Edit) -> Result<Commit, Error> {
        self.change_file(uri, version, edit)
    }

    /// Change one open file to its current content.
    pub fn change_file(&self, uri: Uri, version: i32, edit: Edit) -> Result<Commit, Error> {
        let _write = self.write()?;
        let edit = self.resolve_edit(edit)?;
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

        self.commit_open_file(path, uri, version, edit)
    }

    /// Commit one open file change while its workspace is locked.
    fn commit_open_file(
        &self,
        path: PathBuf,
        uri: Uri,
        version: i32,
        edit: Edit,
    ) -> Result<Commit, Error> {
        if let Some(current) = self.find_open_file(path.as_path()).map(|file| file.version)
            && version <= current
        {
            return Err(Error::StaleOpenFile {
                path,
                incoming: version,
                current,
            });
        }

        // advance the workspace to the open content
        let is_removed = edit.is_remove();
        let revision = self.revision()?;
        let commit = self.commit(revision, vec![edit])?;

        // removed files are no longer open source truth
        if is_removed {
            self.remove_open_file(path.as_path());

            return self.publish(commit);
        }

        // resolve and publish editor identity before notifying semantic watches
        let content = self.content(commit.after, path.as_path())?;
        let file = OpenFile {
            uri,
            version,
            content,
        };
        self.upsert_open_file(path.as_path(), file);

        self.publish(commit)
    }

    /// Patch one open text file.
    pub fn patch_text_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        changes: Vec<TextChange>,
    ) -> Result<Commit, Error> {
        let _write = self.write()?;
        let path = self.resolve_path(path)?;

        // reject stale client versions before computing text
        if let Some(current) = self.find_open_file(&path).map(|file| file.version)
            && version <= current
        {
            return Err(Error::StaleOpenFile {
                path,
                incoming: version,
                current,
            });
        }

        // apply the patch to the current open text
        let Some(file) = self.find_open_file(&path) else {
            return Err(Error::InvalidTextChange {
                path,
                detail: "open file text is not available".to_string(),
            });
        };
        let Content::Text { content: text } = file.content.payload() else {
            return Err(Error::InvalidTextChange {
                path,
                detail: "open file text is not available".to_string(),
            });
        };
        let content = apply_text_changes(text.clone(), &changes).map_err(|error| {
            Error::InvalidTextChange {
                path: path.clone(),
                detail: error.to_string(),
            }
        })?;

        let edit = Edit::SetText {
            path: path.clone(),
            text: content,
        };

        self.commit_open_file(path, uri, version, edit)
    }

    /// Save one open file to explicit content.
    pub fn save_file(&self, edit: Edit) -> Result<Commit, Error> {
        let _write = self.write()?;
        let edit = self.resolve_edit(edit)?;
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

        // advance the workspace to the saved content
        let file = self.find_open_file(path.as_path());
        let is_removed = edit.is_remove();
        let revision = self.revision()?;
        let commit = self.commit(revision, vec![edit])?;

        // remove the open file after committing its removal
        if let Some(file) = file {
            if is_removed {
                self.remove_open_file(path.as_path());

                return self.publish(commit);
            }

            // resolve and publish editor identity before notifying semantic watches
            let content = self.content(commit.after, path.as_path())?;
            let file = OpenFile { content, ..file };
            self.upsert_open_file(path.as_path(), file);

            return self.publish(commit);
        }

        self.publish(commit)
    }

    /// Save text content from explicit content or host filesystem.
    pub fn save_text_file(&self, path: &Path, content: Option<String>) -> Result<Commit, Error> {
        let path = self.resolve_path(path)?;
        let content = match content {
            Some(content) => content,
            None => self
                .repository
                .file_system()
                .read_to_string(&path)
                .map_err(|source| Error::Io {
                    path: path.clone(),
                    source,
                })?,
        };

        self.save_file(Edit::SetText {
            path,
            text: content,
        })
    }

    /// Save binary content from explicit content or host filesystem.
    pub fn save_bytes_file(&self, path: &Path, content: Option<Vec<u8>>) -> Result<Commit, Error> {
        let path = self.resolve_path(path)?;
        let content = match content {
            Some(content) => content,
            None => self
                .repository
                .file_system()
                .read(&path)
                .map_err(|source| Error::Io {
                    path: path.clone(),
                    source,
                })?,
        };

        self.save_file(Edit::SetBytes {
            path,
            bytes: content,
        })
    }

    /// Close one open file and restore filesystem backed source truth.
    pub fn close_file(&self, path: &Path) -> Result<Option<Commit>, Error> {
        let _write = self.write()?;
        let path = self.resolve_path(path)?;
        let revision = self.revision()?;

        // remove overlay state first so filesystem reads see disk truth
        let Some(file) = self.remove_open_file(&path) else {
            return Ok(None);
        };

        // read the current filesystem truth for this path
        let file_system = self.repository.file_system();
        let edit = if FileType::from_path(&path).is_some_and(|file_type| file_type.is_binary()) {
            file_system.read(&path).map(|bytes| Edit::SetBytes {
                path: path.clone(),
                bytes,
            })
        } else {
            file_system.read_to_string(&path).map(|text| Edit::SetText {
                path: path.clone(),
                text,
            })
        };
        let edit = match edit {
            Ok(edit) => edit,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Edit::Remove { path: path.clone() }
            }
            Err(error) => {
                self.upsert_open_file(&path, file);

                return Err(Error::Io {
                    path,
                    source: error,
                });
            }
        };

        // restore the open overlay if the repository update fails
        let commit = match self.commit(revision, vec![edit]) {
            Ok(commit) => self.publish(commit)?,
            Err(error) => {
                self.upsert_open_file(&path, file);

                return Err(error);
            }
        };

        Ok(Some(commit))
    }

    /// Apply one edit through the workspace.
    pub fn apply_file(&self, edit: Edit) -> Result<Commit, Error> {
        let _write = self.write()?;
        let edit = self.resolve_edit(edit)?;

        // apply one direct edit through this workspace
        let revision = self.revision()?;
        let commit = self.commit(revision, vec![edit])?;

        self.publish(commit)
    }

    /// Apply atomic edits through the workspace.
    pub fn apply_source_edits(&self, edits: Vec<Edit>) -> Result<Commit, Error> {
        let _write = self.write()?;
        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(edit))
            .collect::<Result<Vec<_>, _>>()?;

        // publish the edit batch through this workspace
        let revision = self.revision()?;
        let commit = self.commit(revision, edits)?;

        self.publish(commit)
    }

    /// Apply atomic edits when the current revision still matches.
    pub fn apply_source_edits_if_current(
        &self,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, Error> {
        let _write = self.write()?;
        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(edit))
            .collect::<Result<Vec<_>, _>>()?;

        // publish the edit batch through this workspace
        let commit = self.commit(revision, edits)?;

        self.publish(commit)
    }

    /// Apply one editor file operation.
    pub fn apply_file_operation(&self, operation: FileOperation) -> Result<Option<Commit>, Error> {
        match operation {
            FileOperation::OpenText {
                path,
                uri,
                version,
                content,
            } => self
                .open_file(
                    uri,
                    version,
                    Edit::SetText {
                        path,
                        text: content,
                    },
                )
                .map(Some),
            FileOperation::OpenBytes {
                path,
                uri,
                version,
                content,
            } => self
                .open_file(
                    uri,
                    version,
                    Edit::SetBytes {
                        path,
                        bytes: content,
                    },
                )
                .map(Some),
            FileOperation::ChangeText {
                path,
                uri,
                version,
                content,
            } => self
                .change_file(
                    uri,
                    version,
                    Edit::SetText {
                        path,
                        text: content,
                    },
                )
                .map(Some),
            FileOperation::ChangeBytes {
                path,
                uri,
                version,
                content,
            } => self
                .change_file(
                    uri,
                    version,
                    Edit::SetBytes {
                        path,
                        bytes: content,
                    },
                )
                .map(Some),
            FileOperation::PatchText {
                path,
                uri,
                version,
                changes,
            } => self.patch_text_file(&path, uri, version, changes).map(Some),
            FileOperation::SaveText { path, content } => {
                self.save_text_file(&path, content).map(Some)
            }
            FileOperation::SaveBytes { path, content } => {
                self.save_bytes_file(&path, content).map(Some)
            }
            FileOperation::Close { path } => self.close_file(&path),
            FileOperation::WriteText { path, content } => self
                .write_file(Edit::SetText {
                    path,
                    text: content,
                })
                .map(Some),
            FileOperation::WriteBytes { path, content } => self
                .write_file(Edit::SetBytes {
                    path,
                    bytes: content,
                })
                .map(Some),
            FileOperation::Remove { path } => self.write_file(Edit::Remove { path }).map(Some),
            FileOperation::Move { from, to } => self.write_file(Edit::Move { from, to }).map(Some),
        }
    }
}
