use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_repository::Revision;
use destack_session as session;
use destack_session::Session;
use destack_source::{
    FileContentId, FileWatchEvent, FileWatchEventKind, TextChange, Uri, apply_text_changes,
};

use crate::diagnostic::{Error, diagnostics_by_file};
use crate::file::FileUpdate;
use crate::workspace::{Message, UpdateBatch, Workspace};

/// Workspace projection of one committed edit batch.
#[derive(Debug)]
pub struct Commit {
    /// Previous repository revision.
    pub before: Revision,
    /// Updated repository revision.
    pub after: Revision,
    /// Workspace updates produced by the commit.
    pub updates: Vec<FileUpdate>,
    /// Messages produced by the commit.
    pub messages: Vec<Message>,
}

impl Workspace {
    /// Open one file with its current content.
    pub fn open_file(
        &self,
        uri: Uri,
        version: i32,
        edit: session::Edit,
    ) -> Result<UpdateBatch, Error> {
        self.change_file(uri, version, edit)
    }

    /// Change one open file to its current content.
    pub fn change_file(
        &self,
        uri: Uri,
        version: i32,
        edit: session::Edit,
    ) -> Result<UpdateBatch, Error> {
        let path = file_content_edit_path(&edit)?.to_path_buf();

        // reject stale client versions before mutating repository state
        if let Some(current) = self.open_file_version(path.as_path()) {
            if version <= current {
                return Err(Error::StaleOpenFile {
                    path,
                    incoming: version,
                    current,
                });
            }
        }

        // publish the open content as repository source truth
        let session = self.edit_session(path.as_path())?;
        let is_removed = edit.is_remove();
        let open_content = edit.clone();
        let commit = session.edit(session.head(), vec![edit])?;

        // removed files are no longer open source truth
        if is_removed {
            self.remove_open_state(path.as_path());

            return self.build_change_result(&session, commit.changes);
        }

        // store client metadata after the revision carries the same content
        let revision = session.revision(session.head())?;
        let content_id = self.content_id_at_path(&session, revision, path.as_path())?;
        self.set_open_state(path.as_path(), uri, version, content_id, open_content);

        self.build_change_result(&session, commit.changes)
    }

    /// Patch one open text file.
    pub fn patch_text_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        changes: Vec<TextChange>,
    ) -> Result<UpdateBatch, Error> {
        // reject stale client versions before computing text
        if let Some(current) = self.open_file_version(path) {
            if version <= current {
                return Err(Error::StaleOpenFile {
                    path: path.to_path_buf(),
                    incoming: version,
                    current,
                });
            }
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

        self.change_file(
            uri,
            version,
            session::Edit::SetText {
                path: path.to_path_buf(),
                text: content,
            },
        )
    }

    /// Save one open file to explicit content.
    pub fn save_file(&self, edit: session::Edit) -> Result<UpdateBatch, Error> {
        let path = file_content_edit_path(&edit)?.to_path_buf();

        // publish the saved content to the repository revision
        let session = self.edit_session(path.as_path())?;
        let file = self.open_state(path.as_path());
        let is_removed = edit.is_remove();
        let open_content = edit.clone();
        let commit = session.edit(session.head(), vec![edit])?;

        // keep open file protocol metadata when the file remains open
        if let Some(file) = file {
            if is_removed {
                self.remove_open_state(path.as_path());

                return self.build_change_result(&session, commit.changes);
            }

            let revision = session.revision(session.head())?;
            let content_id = self.content_id_at_path(&session, revision, path.as_path())?;

            self.set_open_state(
                path.as_path(),
                file.uri,
                file.version,
                content_id,
                open_content,
            );
        }

        self.build_change_result(&session, commit.changes)
    }

    /// Close one open file and restore filesystem backed source truth.
    pub fn close_file(&self, path: &Path) -> Result<UpdateBatch, Error> {
        // remove overlay state first so filesystem reads see disk truth
        let session = self.edit_session(path)?;
        let Some(file) = self.remove_open_state(path) else {
            return Ok(UpdateBatch::default());
        };

        // read the current filesystem truth for this path
        let edit = match session.read_filesystem_edit(path) {
            Ok(edit) => edit,
            Err(error) if error.kind() == ErrorKind::NotFound => session::Edit::Remove {
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

        self.build_change_result(&session, commit.changes)
    }

    /// Apply one edit through the workspace.
    pub fn apply_file(&self, edit: session::Edit) -> Result<UpdateBatch, Error> {
        let path = file_content_edit_path(&edit)?.to_path_buf();

        // apply direct edits through the owning session
        let session = self.edit_session(path.as_path())?;
        let commit = session
            .edit(session.head(), vec![edit])
            .map_err(Error::from)?;

        self.build_change_result(&session, commit.changes)
    }

    /// Apply atomic edits through the workspace.
    pub fn apply_source_edits(
        &self,
        root: &Path,
        edits: Vec<session::Edit>,
    ) -> Result<Commit, Error> {
        // publish the edit batch through the owning session
        let session = self.session(root)?;
        let commit = session.edit(session.head(), edits)?;
        self.workspace_commit(&session, commit)
    }

    /// Apply atomic edits when the current revision still matches.
    pub fn apply_source_edits_at(
        &self,
        root: &Path,
        revision: Revision,
        edits: Vec<session::Edit>,
    ) -> Result<Commit, Error> {
        // publish the edit batch through the owning session
        let session = self.session(root)?;
        let commit = session.edit_at(session.head(), revision, edits)?;

        self.workspace_commit(&session, commit)
    }

    /// Build one workspace commit from one session commit.
    fn workspace_commit(
        &self,
        session: &Session,
        commit: session::Commit,
    ) -> Result<Commit, Error> {
        let before = commit.before;
        let after = commit.after;
        let result = self.build_change_result(session, commit.changes)?;

        Ok(Commit {
            before,
            after,
            updates: result.updates,
            messages: result.messages,
        })
    }

    /// Apply watch events through the workspace.
    pub fn apply_watch_events(&self, events: Vec<FileWatchEvent>) -> Result<UpdateBatch, Error> {
        // apply precise watch events before escalating to a full reload
        let (mut result, require_reload) = self.apply_watch_events_immediate(events)?;

        if require_reload {
            let reload = self.reload_all()?;
            result.updates.extend(reload.updates);
            result.messages.extend(reload.messages);
        }

        Ok(result)
    }

    /// Apply watch events and return immediate updates plus reload intent.
    fn apply_watch_events_immediate(
        &self,
        events: Vec<FileWatchEvent>,
    ) -> Result<(UpdateBatch, bool), Error> {
        // empty batches do no work
        let mut result = UpdateBatch::default();
        if events.is_empty() {
            return Ok((result, false));
        }

        let mut require_reload = false;

        // fold events into direct updates and reload intent
        for event in events {
            let is_path_open = self.has_open_file(&event.path);
            let is_previous_path_open = event
                .previous_path
                .as_ref()
                .is_some_and(|path| self.has_open_file(path));
            if is_path_open || is_previous_path_open {
                continue;
            }

            // overflow means the event stream is incomplete
            if matches!(event.kind, FileWatchEventKind::Overflow) {
                require_reload = true;
                result.messages.push(Message::warning(
                    "watch_overflow_reload",
                    "watch: filesystem reload required after overflow",
                ));
                continue;
            }

            // deletion can invalidate import closure membership
            if matches!(event.kind, FileWatchEventKind::Deleted) {
                require_reload = true;

                self.apply_watch_removal(&event.path, &mut result)?;

                continue;
            }

            // rename is a removal plus a new source path
            if matches!(event.kind, FileWatchEventKind::Renamed) {
                require_reload = true;

                if let Some(previous_path) = event.previous_path.as_ref() {
                    self.apply_watch_removal(previous_path, &mut result)?;
                }

                self.apply_watch_file(&event.path, &mut result)?;

                continue;
            }

            // creation can add new import candidates
            if matches!(event.kind, FileWatchEventKind::Created) {
                require_reload = true;
            }

            self.apply_watch_file(&event.path, &mut result)?;
        }

        Ok((result, require_reload))
    }

    /// Apply one watched file removal.
    fn apply_watch_removal(&self, path: &Path, result: &mut UpdateBatch) -> Result<(), Error> {
        // ignore paths outside repository reload policy
        let session = self.edit_session(path)?;
        if !session.imports_filesystem_path(path) {
            return Ok(());
        }

        // publish the removal when the file was tracked
        match session.edit(
            session.head(),
            vec![session::Edit::Remove {
                path: path.to_path_buf(),
            }],
        ) {
            Ok(commit) => self.extend_with_change_result(&session, commit.changes, result)?,
            Err(error) => result.messages.push(Message::warning(
                "watch_remove_failed",
                format!("watch: failed to remove {}: {error}", path.display()),
            )),
        }

        Ok(())
    }

    /// Apply one watched file content refresh.
    fn apply_watch_file(&self, path: &Path, result: &mut UpdateBatch) -> Result<(), Error> {
        // ignore paths outside repository reload policy
        let session = self.edit_session(path)?;
        if !session.imports_filesystem_path(path) {
            return Ok(());
        }

        // read the latest filesystem payload
        let edit = match session.read_filesystem_edit(path) {
            Ok(edit) => edit,
            Err(error) => {
                result.messages.push(Message::warning(
                    "watch_read_failed",
                    format!("watch: failed to read {}: {error}", path.display()),
                ));

                return Ok(());
            }
        };

        // publish the refreshed file payload
        match session.edit(session.head(), vec![edit]) {
            Ok(commit) => self.extend_with_change_result(&session, commit.changes, result)?,
            Err(error) => result.messages.push(Message::warning(
                "watch_update_failed",
                format!("watch: failed to update {}: {error}", path.display()),
            )),
        }

        Ok(())
    }

    /// Extend one workspace result with one session change list.
    fn extend_with_change_result(
        &self,
        session: &Session,
        changes: Vec<session::Change>,
        result: &mut UpdateBatch,
    ) -> Result<(), Error> {
        // rebuild workspace updates from the committed session changes
        let update = self.build_change_result(session, changes)?;

        result.updates.extend(update.updates);
        result.messages.extend(update.messages);

        Ok(())
    }

    /// Reload filesystem state for every root.
    pub fn reload_all(&self) -> Result<UpdateBatch, Error> {
        // snapshot roots before mutating sessions
        let roots = self.root_paths();

        self.reload_roots(&roots)
    }

    /// Reload filesystem state for specific roots.
    pub fn reload_roots(&self, roots: &[PathBuf]) -> Result<UpdateBatch, Error> {
        let mut result = UpdateBatch::default();

        // reload each root independently
        for root in roots {
            let session = self.session(root)?;
            let open_files = self.open_files_under(root);
            let changes = session.reload_from_fs(session.head())?;
            let update = self.build_change_result(&session, changes)?;

            result.updates.extend(update.updates);
            result.messages.extend(update.messages);

            // reapply open file source truth after filesystem reloads
            for (path, file) in open_files {
                let commit = session.edit(session.head(), vec![file.content.clone()])?;
                let revision = session.revision(session.head())?;
                let content_id = self.content_id_at_path(&session, revision, path.as_path())?;
                let update = self.build_change_result(&session, commit.changes)?;

                self.set_open_state(
                    path.as_path(),
                    file.uri.clone(),
                    file.version,
                    content_id,
                    file.content.clone(),
                );
                result.updates.extend(update.updates);
                result.messages.extend(update.messages);
            }
        }

        Ok(result)
    }

    /// Build one workspace result for committed session changes.
    fn build_change_result(
        &self,
        session: &Session,
        changes: Vec<session::Change>,
    ) -> Result<UpdateBatch, Error> {
        // convert session changes before requesting derived artifacts
        let revision = session.revision(session.head())?;
        let mut updates = self.file_updates(session, revision, changes)?;

        // attach diagnostics after the source revision is sealed
        self.attach_diagnostics(session, revision, &mut updates)?;

        Ok(UpdateBatch::from(updates))
    }

    /// Convert session changes into workspace file updates.
    fn file_updates(
        &self,
        session: &Session,
        revision: Revision,
        changes: Vec<session::Change>,
    ) -> Result<Vec<FileUpdate>, Error> {
        let mut file_updates = Vec::new();

        // project each session change into workspace protocol shape
        for change in changes {
            file_updates.push(self.file_update(session, revision, change)?);
        }

        Ok(file_updates)
    }

    /// Convert one session change into a workspace file update.
    fn file_update(
        &self,
        session: &Session,
        revision: Revision,
        change: session::Change,
    ) -> Result<FileUpdate, Error> {
        let mut update = FileUpdate::from(change);

        // attach open file protocol identity when the revision content agrees
        if let Some(path) = update.file.as_ref().and_then(|file| file.path.as_deref()) {
            if let Some(file) = self.open_state(path) {
                update.diagnostic_uri = file.uri;
                update.diagnostic_version = self.open_file_version_in_revision(
                    session.repository().as_ref(),
                    revision,
                    update.file_id,
                    path,
                )?;
            }
        }

        Ok(update)
    }

    /// Return the content identity for one path in a revision.
    fn content_id_at_path(
        &self,
        session: &Session,
        revision: Revision,
        path: &Path,
    ) -> Result<FileContentId, Error> {
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

    /// Attach current diagnostics to each changed file update.
    fn attach_diagnostics(
        &self,
        session: &Session,
        revision: Revision,
        updates: &mut [FileUpdate],
    ) -> Result<(), Error> {
        // consume diagnostics by primary source file
        let mut diagnostics = diagnostics_by_file(session.repository().as_ref(), revision)?;

        for update in updates {
            update.diagnostics = diagnostics.remove(&update.file_id).unwrap_or_default();
        }

        Ok(())
    }
}

/// Return the path for a full content edit.
fn file_content_edit_path(edit: &session::Edit) -> Result<&Path, Error> {
    match edit {
        session::Edit::SetText { path, .. }
        | session::Edit::SetBytes { path, .. }
        | session::Edit::Remove { path } => Ok(path.as_path()),
        session::Edit::EditText { .. } => Err(Error::InvalidEdit {
            detail: "text patch edits are only valid in edit batches".to_string(),
        }),
        session::Edit::Move { .. } => Err(Error::InvalidEdit {
            detail: "move edits are only valid in edit batches".to_string(),
        }),
    }
}
