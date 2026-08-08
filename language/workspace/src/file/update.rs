use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_repository::Revision;
use destack_serde::Reflect;
use destack_session as session;
use destack_session::{Change, Session, SessionError};
use destack_source::{ContentId, Edit, TextChange, Uri, apply_text_changes};
use serde::{Deserialize, Serialize};

use crate::diagnostic::Error;
use crate::file::FileUpdate;
use crate::workspace::{LocalWorkspace, WorkspaceRoot};

/// One requested source mutation batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SourceUpdate {
    /// Optional expected base revision.
    pub base: Option<Revision>,
    /// Source file edits.
    pub edits: Vec<Edit>,
}

/// Workspace projection of one committed edit batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Commit {
    /// Previous repository revision.
    pub before: Revision,
    /// Updated repository revision.
    pub after: Revision,
    /// Workspace updates produced by the commit.
    pub updates: Vec<FileUpdate>,
}

/// Previous filesystem content retained until one source commit publishes.
struct FileBackup {
    /// The source path.
    path: PathBuf,
    /// The previous bytes, or `None` when the path did not exist.
    content: Option<Vec<u8>>,
}

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
        let root = self.path_root(path.as_path())?;
        let _write = root.writes.lock();

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
        let session = &root.session;

        if let Some(current) = self.open_file_version(path.as_path())
            && version <= current
        {
            return Err(Error::StaleOpenFile {
                path,
                incoming: version,
                current,
            });
        }

        // publish the open content as repository source truth
        let is_removed = edit.is_remove();
        let open_content = edit.clone();
        let commit = session.edit(session.head(), vec![edit])?;

        // removed files are no longer open source truth
        if is_removed {
            self.remove_open_state(path.as_path());

            return Ok(self.workspace_commit(root, commit));
        }

        // store client metadata after the revision carries the same content
        let revision = session.revision(session.head())?;
        let content_id = self.content_id_at_path(session, revision, path.as_path())?;
        self.set_open_state(path.as_path(), uri, version, content_id, open_content);

        Ok(self.workspace_commit(root, commit))
    }

    /// Patch one open text file.
    pub fn patch_text_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        changes: Vec<TextChange>,
    ) -> Result<Commit, Error> {
        let root = self.path_root(path)?;
        let _write = root.writes.lock();

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
        let root = self.path_root(path.as_path())?;
        let _write = root.writes.lock();
        let session = &root.session;

        // publish the saved content to the repository revision
        let file = self.open_state(path.as_path());
        let is_removed = edit.is_remove();
        let open_content = edit.clone();
        let commit = session.edit(session.head(), vec![edit])?;

        // keep open file protocol metadata when the file remains open
        if let Some(file) = file {
            if is_removed {
                self.remove_open_state(path.as_path());

                return Ok(self.workspace_commit(root.as_ref(), commit));
            }

            let revision = session.revision(session.head())?;
            let content_id = self.content_id_at_path(session, revision, path.as_path())?;

            self.set_open_state(
                path.as_path(),
                file.uri,
                file.version,
                content_id,
                open_content,
            );
        }

        Ok(self.workspace_commit(root.as_ref(), commit))
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
        let root = self.path_root(path)?;
        let _write = root.writes.lock();
        let session = &root.session;

        // remove overlay state first so filesystem reads see disk truth
        let Some(file) = self.remove_open_state(path) else {
            return Ok(None);
        };

        // read the current filesystem truth for this path
        let edit = match session.read_filesystem_edit(path) {
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
        let commit = match session.edit(session.head(), vec![edit]) {
            Ok(commit) => commit,
            Err(error) => {
                self.set_open_state(path, file.uri, file.version, file.content_id, file.content);

                return Err(Error::from(error));
            }
        };

        Ok(Some(self.workspace_commit(root.as_ref(), commit)))
    }

    /// Apply one edit through the workspace.
    pub fn apply_file(&self, edit: Edit) -> Result<Commit, Error> {
        let root = self.edit_root(&edit)?;
        let _write = root.writes.lock();
        let session = &root.session;
        let edit = self.resolve_edit(root.as_ref(), edit)?;

        // apply direct edits through the owning session
        let commit = session
            .edit(session.head(), vec![edit])
            .map_err(Error::from)?;

        Ok(self.workspace_commit(root.as_ref(), commit))
    }

    /// Write one edit to the host filesystem and workspace.
    pub fn write_file(&self, edit: Edit) -> Result<Commit, Error> {
        let root = self.edit_root(&edit)?;
        let _write = root.writes.lock();
        let session = &root.session;
        let edit = self.resolve_edit(root.as_ref(), edit)?;
        let revision = session.revision(session.head())?;
        let commit = self.commit_source_edits(session, revision, vec![edit])?;

        Ok(self.workspace_commit(root.as_ref(), commit))
    }

    /// Write source edits when the current revision still matches.
    pub(crate) fn write_source_edits_if_current(
        &self,
        root: &Path,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, Error> {
        let root = self.workspace_root(root)?;
        let _write = root.writes.lock();
        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(root.as_ref(), edit))
            .collect::<Result<Vec<_>, _>>()?;
        let commit = self.commit_source_edits(&root.session, revision, edits)?;

        Ok(self.workspace_commit(root.as_ref(), commit))
    }

    /// Apply atomic edits through the workspace.
    pub fn apply_source_edits(&self, root: &Path, edits: Vec<Edit>) -> Result<Commit, Error> {
        let root = self.workspace_root(root)?;
        let _write = root.writes.lock();
        let session = &root.session;
        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(root.as_ref(), edit))
            .collect::<Result<Vec<_>, _>>()?;

        // publish the edit batch through the owning session
        let commit = session.edit(session.head(), edits)?;

        Ok(self.workspace_commit(root.as_ref(), commit))
    }

    /// Apply atomic edits when the current revision still matches.
    pub fn apply_source_edits_if_current(
        &self,
        root: &Path,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, Error> {
        let root = self.workspace_root(root)?;
        let _write = root.writes.lock();
        let session = &root.session;
        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(root.as_ref(), edit))
            .collect::<Result<Vec<_>, _>>()?;

        // publish the edit batch through the owning session
        let commit = session.edit_if_current(session.head(), revision, edits)?;

        Ok(self.workspace_commit(root.as_ref(), commit))
    }

    /// Commit source edits to disk and one exact session revision.
    fn commit_source_edits(
        &self,
        session: &Session,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<session::Commit, Error> {
        // retain disk truth before preparing repository state
        let backups = self.backup_files(&edits)?;

        // build and pin the edited revision without publishing it
        let prepared = match session.prepare_edit(session.head(), revision, edits.clone()) {
            Ok(prepared) => prepared,
            Err(SessionError::StaleRevision {
                expected, current, ..
            }) => return Err(Error::StaleRevision { expected, current }),
            Err(error) => return Err(Error::from(error)),
        };

        // write every file while the root and session remain locked
        for edit in &edits {
            if let Err(error) = self.write_update_to_disk(edit) {
                return Err(self.restore_after(error, &backups));
            }
        }

        // publish only after every filesystem write succeeds
        match prepared.publish() {
            Ok(commit) => Ok(commit),
            Err(error) => Err(self.restore_after(Error::from(error), &backups)),
        }
    }

    /// Retain original filesystem bytes for every edited path.
    fn backup_files(&self, edits: &[Edit]) -> Result<Vec<FileBackup>, Error> {
        let mut paths = HashSet::new();
        let mut backups = Vec::new();

        // read every distinct path before performing any write
        for edit in edits {
            match edit {
                Edit::SetText { path, .. }
                | Edit::SetBytes { path, .. }
                | Edit::Remove { path } => {
                    self.backup_file(path, &mut paths, &mut backups)?;
                }
                Edit::Move { from, to } => {
                    if from == to {
                        return Err(Error::InvalidEdit {
                            detail: "move source and destination must differ".to_string(),
                        });
                    }

                    self.backup_file(from, &mut paths, &mut backups)?;
                    self.backup_file(to, &mut paths, &mut backups)?;
                }
                Edit::EditText { .. } => {
                    return Err(Error::InvalidEdit {
                        detail: "text patch edits cannot be written directly to disk".to_string(),
                    });
                }
            }
        }

        Ok(backups)
    }

    /// Retain one original filesystem path once.
    fn backup_file(
        &self,
        path: &Path,
        paths: &mut HashSet<PathBuf>,
        backups: &mut Vec<FileBackup>,
    ) -> Result<(), Error> {
        // reject disk writes while editor content owns the path
        if self.has_open_file(path) {
            return Err(Error::OpenFileWrite {
                path: path.to_path_buf(),
            });
        }

        // retain each physical path once
        if !paths.insert(path.to_path_buf()) {
            return Ok(());
        }

        // snapshot the current bytes or absence
        let file_system = self.repository.file_system();
        let content = if file_system.exists(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })? {
            Some(file_system.read(path).map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?)
        } else {
            None
        };
        backups.push(FileBackup {
            path: path.to_path_buf(),
            content,
        });

        Ok(())
    }

    /// Restore original filesystem content after a failed source commit.
    fn restore_after(&self, operation: Error, backups: &[FileBackup]) -> Error {
        // restore every path even when one restoration fails
        let mut failure = None;
        for backup in backups.iter().rev() {
            let result = match &backup.content {
                Some(content) => self.repository.file_system().write(&backup.path, content),
                None => self.repository.file_system().remove_file(&backup.path),
            };
            if let Err(source) = result
                && source.kind() != ErrorKind::NotFound
                && failure.is_none()
            {
                failure = Some((backup.path.clone(), source));
            }
        }

        match failure {
            Some((path, source)) => Error::RollbackFailed {
                operation: Box::new(operation),
                path,
                source,
            },
            None => operation,
        }
    }

    /// Build one workspace commit from one session commit.
    fn workspace_commit(&self, root: &WorkspaceRoot, commit: session::Commit) -> Commit {
        let before = commit.before;
        let after = commit.after;
        let updates = self.file_updates(commit.changes);
        let commit = Commit {
            before,
            after,
            updates,
        };

        // publish only real revision transitions
        if commit.before != commit.after {
            root.watch.lock().publish(&commit);
        }

        commit
    }

    /// Reload one root from its host filesystem.
    pub fn reload_root(&self, root_path: &Path) -> Result<Option<Commit>, Error> {
        let root = self.workspace_root(root_path)?;
        let _write = root.writes.lock();
        let session = &root.session;

        // import host source and editor overlays as one revision
        let before = session.revision(session.head())?;
        let changes = session.reload_from_fs(session.head())?;
        let after = session.revision(session.head())?;
        root.watch.lock().recover();
        if before == after {
            return Ok(None);
        }
        let commit = session::Commit {
            before,
            after,
            changes,
        };

        Ok(Some(self.workspace_commit(root.as_ref(), commit)))
    }

    /// Convert session changes into workspace file updates.
    fn file_updates(&self, changes: Vec<Change>) -> Vec<FileUpdate> {
        let mut file_updates = Vec::with_capacity(changes.len());

        // project each session change into workspace protocol shape
        for change in changes {
            file_updates.push(self.file_update(change));
        }

        file_updates
    }

    /// Convert one session change into a workspace file update.
    fn file_update(&self, change: Change) -> FileUpdate {
        let open_file = match &change {
            Change::Updated { file, .. } => file
                .path
                .as_deref()
                .and_then(|path| self.open_state(path))
                .filter(|open| open.content_id == file.content.content_id()),
            Change::Removed { .. } => None,
        };
        let mut update = FileUpdate::from(change);

        // attach open file identity when its exact content agrees
        if let Some(file) = open_file {
            update.diagnostic_uri = file.uri;
            update.diagnostic_version = Some(file.version);
        }

        update
    }

    /// Return the content identity for one path in a revision.
    fn content_id_at_path(
        &self,
        session: &Session,
        revision: Revision,
        path: &Path,
    ) -> Result<ContentId, Error> {
        // open files must point at an existing file payload
        let repository = session.repository();
        let file_id = session.file_id(path);
        let content_id = repository
            .file_content_id(revision, file_id)?
            .ok_or(Error::Internal {
                detail: format!("file has no content in revision: {}", path.display()),
            })?;

        Ok(content_id)
    }

    /// Write an edit to disk before applying it.
    fn write_update_to_disk(&self, edit: &Edit) -> Result<(), Error> {
        match edit {
            Edit::SetText { path, text } => {
                self.create_parent_directory(path)?;
                self.repository
                    .file_system()
                    .write_string(path, text)
                    .map_err(|source| Error::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
            }
            Edit::SetBytes { path, bytes } => {
                self.create_parent_directory(path)?;
                self.repository
                    .file_system()
                    .write(path, bytes)
                    .map_err(|source| Error::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
            }
            Edit::Remove { path } => {
                if let Err(source) = self.repository.file_system().remove_file(path)
                    && source.kind() != ErrorKind::NotFound
                {
                    return Err(Error::Io {
                        path: path.to_path_buf(),
                        source,
                    });
                }
            }
            Edit::EditText { .. } => {
                return Err(Error::InvalidEdit {
                    detail: "text patch edits cannot be written directly to disk".to_string(),
                });
            }
            Edit::Move { from, to } => {
                let bytes =
                    self.repository
                        .file_system()
                        .read(from)
                        .map_err(|source| Error::Io {
                            path: from.to_path_buf(),
                            source,
                        })?;
                self.create_parent_directory(to)?;
                self.repository
                    .file_system()
                    .write(to, &bytes)
                    .map_err(|source| Error::Io {
                        path: to.to_path_buf(),
                        source,
                    })?;
                self.repository
                    .file_system()
                    .remove_file(from)
                    .map_err(|source| Error::Io {
                        path: from.to_path_buf(),
                        source,
                    })?;
            }
        }

        Ok(())
    }

    /// Create the parent directory for one file path.
    fn create_parent_directory(&self, path: &Path) -> Result<(), Error> {
        let Some(parent) = path.parent() else {
            return Ok(());
        };

        self.repository
            .file_system()
            .create_dir_all(parent)
            .map_err(|source| Error::Io {
                path: parent.to_path_buf(),
                source,
            })
    }
}
