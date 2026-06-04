use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_session::{
    FileUpdate as SessionFileUpdate, Session, SourceUpdate as SessionSourceUpdate,
};
use destack_source::{
    FileContentId, FileWatchEvent, FileWatchEventKind, TextChange, Uri, apply_text_changes,
};
use destack_workspace::Revision;

use super::diagnostic::diagnostics_by_file;
use super::{
    FileChange, FileUpdate, LanguageService, LanguageServiceError, LanguageServiceMessage,
    LanguageServiceResult,
};

/// Result of applying one source update.
#[derive(Debug)]
pub struct SourceUpdateResult {
    /// Previous repository revision.
    pub before: Revision,
    /// Updated repository revision.
    pub after: Revision,
    /// Service updates produced by the source update.
    pub updates: Vec<FileUpdate>,
    /// Messages produced by the source update.
    pub messages: Vec<LanguageServiceMessage>,
}

impl LanguageService {
    /// Open one file with its current content.
    pub fn open_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        change: FileChange,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        self.change_file(path, uri, version, change)
    }

    /// Change one open file to its current content.
    pub fn change_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        change: FileChange,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // reject stale client versions before mutating repository state
        if let Some(current) = self.open_file_version(path) {
            if version <= current {
                return Err(LanguageServiceError::StaleOpenFile {
                    path: path.to_path_buf(),
                    incoming: version,
                    current,
                });
            }
        }

        // publish the open content as repository source truth
        let session = self.edit_session(path)?;
        let is_removed = matches!(change, FileChange::Removed);
        let open_content = change.clone();
        let update = session.apply_file(session.head(), path, change)?;

        // removed files are no longer open source truth
        if is_removed {
            self.remove_open_state(path);

            return self.build_change_result(&session, update);
        }

        // store client metadata after the revision carries the same content
        let revision = session.revision(session.head())?;
        let content_id = self.content_id_at_path(&session, revision, path)?;
        self.set_open_state(path, uri, version, content_id, open_content);

        self.build_change_result(&session, update)
    }

    /// Patch one open text file.
    pub fn patch_text_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        changes: Vec<TextChange>,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // reject stale client versions before computing text
        if let Some(current) = self.open_file_version(path) {
            if version <= current {
                return Err(LanguageServiceError::StaleOpenFile {
                    path: path.to_path_buf(),
                    incoming: version,
                    current,
                });
            }
        }

        // apply the patch to the current open text
        let Some(text) = self.open_file_text(path) else {
            return Err(LanguageServiceError::InvalidTextChange {
                path: path.to_path_buf(),
                detail: "open file text is not available".to_string(),
            });
        };
        let content = apply_text_changes(text, &changes).map_err(|error| {
            LanguageServiceError::InvalidTextChange {
                path: path.to_path_buf(),
                detail: error.to_string(),
            }
        })?;

        self.change_file(path, uri, version, FileChange::Text { content })
    }

    /// Save one open file to explicit content.
    pub fn save_file(
        &self,
        path: &Path,
        change: FileChange,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // publish the saved content to the repository revision
        let session = self.edit_session(path)?;
        let file = self.open_state(path);
        let is_removed = matches!(change, FileChange::Removed);
        let open_content = change.clone();
        let update = session.apply_file(session.head(), path, change)?;

        // keep open file protocol metadata when the file remains open
        if let Some(file) = file {
            if is_removed {
                self.remove_open_state(path);

                return self.build_change_result(&session, update);
            }

            let revision = session.revision(session.head())?;
            let content_id = self.content_id_at_path(&session, revision, path)?;

            self.set_open_state(path, file.uri, file.version, content_id, open_content);
        }

        self.build_change_result(&session, update)
    }

    /// Close one open file and restore filesystem backed source truth.
    pub fn close_file(&self, path: &Path) -> Result<LanguageServiceResult, LanguageServiceError> {
        // remove overlay state first so filesystem reads see disk truth
        let session = self.edit_session(path)?;
        let Some(file) = self.remove_open_state(path) else {
            return Ok(LanguageServiceResult::default());
        };

        // read the current filesystem truth for this path
        let update = match session.read_file_from_fs(path) {
            Ok(update) => update,
            Err(error) if error.kind() == ErrorKind::NotFound => FileChange::Removed,
            Err(error) => {
                self.set_open_state(path, file.uri, file.version, file.content_id, file.content);

                return Err(LanguageServiceError::Io {
                    path: path.to_path_buf(),
                    source: error,
                });
            }
        };

        // restore the open overlay if the repository update fails
        let change = match session.apply_file(session.head(), path, update) {
            Ok(change) => change,
            Err(error) => {
                self.set_open_state(path, file.uri, file.version, file.content_id, file.content);

                return Err(LanguageServiceError::from(error));
            }
        };

        self.build_change_result(&session, change)
    }

    /// Apply one file update through the service.
    pub fn apply_file(
        &self,
        path: &Path,
        update: FileChange,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // apply direct source edits through the owning session
        let session = self.edit_session(path)?;
        let change = session
            .apply_file(session.head(), path, update)
            .map_err(LanguageServiceError::from)?;

        self.build_change_result(&session, change)
    }

    /// Apply one atomic source update through the service.
    pub fn apply_source_update(
        &self,
        root: &Path,
        update: SessionSourceUpdate,
    ) -> Result<SourceUpdateResult, LanguageServiceError> {
        // publish the source batch through the owning session
        let session = self.session(root)?;
        let update = session.update(session.head(), update)?;
        let before = update.before;
        let after = update.after;
        let result = self.build_change_result(&session, update.files)?;

        Ok(SourceUpdateResult {
            before,
            after,
            updates: result.updates,
            messages: result.messages,
        })
    }

    /// Apply watch events through the service.
    pub fn apply_watch_events(
        &self,
        events: Vec<FileWatchEvent>,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
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
    ) -> Result<(LanguageServiceResult, bool), LanguageServiceError> {
        // empty batches do no work
        let mut result = LanguageServiceResult::default();
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
                result.messages.push(LanguageServiceMessage::warning(
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
    fn apply_watch_removal(
        &self,
        path: &Path,
        result: &mut LanguageServiceResult,
    ) -> Result<(), LanguageServiceError> {
        // ignore paths outside repository reload policy
        let session = self.edit_session(path)?;
        if !session.is_reload_path(path) {
            return Ok(());
        }

        // publish the removal when the file was tracked
        match session.apply_file(session.head(), path, FileChange::Removed) {
            Ok(change) => self.extend_with_change_result(&session, change, result)?,
            Err(error) => result.messages.push(LanguageServiceMessage::warning(
                "watch_remove_failed",
                format!("watch: failed to remove {}: {error}", path.display()),
            )),
        }

        Ok(())
    }

    /// Apply one watched file content refresh.
    fn apply_watch_file(
        &self,
        path: &Path,
        result: &mut LanguageServiceResult,
    ) -> Result<(), LanguageServiceError> {
        // ignore paths outside repository reload policy
        let session = self.edit_session(path)?;
        if !session.is_reload_path(path) {
            return Ok(());
        }

        // read the latest filesystem payload
        let update = match session.read_file_from_fs(path) {
            Ok(update) => update,
            Err(error) => {
                result.messages.push(LanguageServiceMessage::warning(
                    "watch_read_failed",
                    format!("watch: failed to read {}: {error}", path.display()),
                ));

                return Ok(());
            }
        };

        // publish the refreshed file payload
        match session.apply_file(session.head(), path, update) {
            Ok(change) => self.extend_with_change_result(&session, change, result)?,
            Err(error) => result.messages.push(LanguageServiceMessage::warning(
                "watch_update_failed",
                format!("watch: failed to update {}: {error}", path.display()),
            )),
        }

        Ok(())
    }

    /// Extend one service result with one session change result.
    fn extend_with_change_result(
        &self,
        session: &Session,
        change: Vec<SessionFileUpdate>,
        result: &mut LanguageServiceResult,
    ) -> Result<(), LanguageServiceError> {
        // rebuild service updates from the committed repository change
        let update = self.build_change_result(session, change)?;

        result.updates.extend(update.updates);
        result.messages.extend(update.messages);

        Ok(())
    }

    /// Reload filesystem state for every root.
    pub fn reload_all(&self) -> Result<LanguageServiceResult, LanguageServiceError> {
        // snapshot roots before mutating sessions
        let roots: Vec<PathBuf> = self.roots.iter().map(|entry| entry.key().clone()).collect();

        self.reload_roots(&roots)
    }

    /// Reload filesystem state for specific roots.
    pub fn reload_roots(
        &self,
        roots: &[PathBuf],
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let mut result = LanguageServiceResult::default();

        // reload each root independently
        for root in roots {
            let session = self.session(root)?;
            let open_files = self.open_files_under(root);
            let change = session.reload_from_fs(session.head())?;
            let update = self.build_change_result(&session, change)?;

            result.updates.extend(update.updates);
            result.messages.extend(update.messages);

            // reapply open file source truth after filesystem reloads
            for (path, file) in open_files {
                let change =
                    session.apply_file(session.head(), path.as_path(), file.content.clone())?;
                let revision = session.revision(session.head())?;
                let content_id = self.content_id_at_path(&session, revision, path.as_path())?;
                let update = self.build_change_result(&session, change)?;

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

    /// Build one service result for a committed session change.
    fn build_change_result(
        &self,
        session: &Session,
        updates: Vec<SessionFileUpdate>,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // convert source updates before requesting derived artifacts
        let revision = session.revision(session.head())?;
        let mut updates = self.file_updates(session, revision, updates)?;

        // attach diagnostics after the source revision is sealed
        self.attach_diagnostics(session, revision, &mut updates)?;

        Ok(LanguageServiceResult::from(updates))
    }

    /// Convert session file updates into service file updates.
    fn file_updates(
        &self,
        session: &Session,
        revision: Revision,
        updates: Vec<SessionFileUpdate>,
    ) -> Result<Vec<FileUpdate>, LanguageServiceError> {
        let mut file_updates = Vec::new();

        // project each session update into service protocol shape
        for update in updates {
            file_updates.push(self.file_update(session, revision, update)?);
        }

        Ok(file_updates)
    }

    /// Convert one session file update into a service file update.
    fn file_update(
        &self,
        session: &Session,
        revision: Revision,
        update: SessionFileUpdate,
    ) -> Result<FileUpdate, LanguageServiceError> {
        let mut update = FileUpdate::from(update);

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
    ) -> Result<FileContentId, LanguageServiceError> {
        // open files must point at an existing file payload
        let repository = session.repository();
        let file_id = session.file_id(path);
        let content_id = repository.file_content_id(revision, file_id)?.ok_or(
            LanguageServiceError::Internal {
                detail: format!("file has no content in revision: {}", path.display()),
            },
        )?;

        Ok(content_id)
    }

    /// Attach current diagnostics to each changed file update.
    fn attach_diagnostics(
        &self,
        session: &Session,
        revision: Revision,
        updates: &mut [FileUpdate],
    ) -> Result<(), LanguageServiceError> {
        // consume diagnostics by primary source file
        let mut diagnostics = diagnostics_by_file(session.repository().as_ref(), revision)?;

        for update in updates {
            update.diagnostics = diagnostics.remove(&update.file_id).unwrap_or_default();
        }

        Ok(())
    }
}
