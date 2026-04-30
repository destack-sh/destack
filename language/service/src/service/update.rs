use std::path::{Path, PathBuf};

use destack_artifact::ArtifactKey;
use destack_session::{FileUpdate as SessionFileUpdate, Session, SessionChange};
use destack_source::{Diagnostic, FileWatchEvent, FileWatchEventKind, Uri};
use destack_workspace::{Repository, Revision};

use super::{
    FileMutation, FileUpdate, LanguageService, LanguageServiceError, LanguageServiceMessage,
    LanguageServiceResult,
};

impl LanguageService {
    /// Open one file with its current text.
    pub fn open_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        text: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        self.update_open_file(path, uri, version, text)
    }

    /// Change one open file to its current text.
    pub fn change_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        text: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        self.update_open_file(path, uri, version, text)
    }

    /// Update one open file to its current text.
    fn update_open_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        text: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.edit_session(path)?;
        let change = session
            .update_open_file(session.head(), path, uri, version, text)
            .map_err(LanguageServiceError::from)?;

        self.build_change_result(path, change)
    }

    /// Save one open file to explicit saved text.
    pub fn save_file(
        &self,
        path: &Path,
        text: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.edit_session(path)?;
        let change = session
            .save_open_file(session.head(), path, text)
            .map_err(LanguageServiceError::from)?;

        self.build_change_result(path, change)
    }

    /// Close one open file and restore filesystem backed source truth.
    pub fn close_file(&self, path: &Path) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.edit_session(path)?;
        let change = session
            .close_open_file(session.head(), path)
            .map_err(LanguageServiceError::from)?;

        self.build_change_result(path, change)
    }

    /// Apply one file update through the service.
    pub fn apply_file(
        &self,
        path: &Path,
        update: FileMutation,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.edit_session(path)?;
        let change = session
            .apply(session.head(), path, update)
            .map_err(LanguageServiceError::from)?;

        self.build_change_result(path, change)
    }

    /// Apply watch events through the service.
    pub fn apply_watch_events(
        &self,
        events: Vec<FileWatchEvent>,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
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
        let mut result = LanguageServiceResult::default();
        if events.is_empty() {
            return Ok((result, false));
        }

        let mut require_reload = false;

        for event in events {
            let is_path_open = self.has_open_file(&event.path);
            let is_previous_path_open = event
                .previous_path
                .as_ref()
                .is_some_and(|path| self.has_open_file(path));
            if is_path_open || is_previous_path_open {
                continue;
            }

            if matches!(event.kind, FileWatchEventKind::Overflow) {
                require_reload = true;
                result.messages.push(LanguageServiceMessage::warning(
                    "watch_overflow_reload",
                    "watch: filesystem reload required after overflow",
                ));
                continue;
            }

            if matches!(event.kind, FileWatchEventKind::Deleted) {
                require_reload = true;

                let session = self.edit_session(&event.path)?;
                if !session.should_watch_path(&event.path) {
                    continue;
                }

                match session.apply(session.head(), &event.path, FileMutation::Removed) {
                    Ok(change) => {
                        let updates = self.file_updates(&session, change.after, change.files)?;

                        result.updates.extend(updates);
                    }
                    Err(error) => result.messages.push(LanguageServiceMessage::warning(
                        "watch_remove_failed",
                        format!("watch: failed to remove {}: {error}", event.path.display()),
                    )),
                }

                continue;
            }

            if matches!(event.kind, FileWatchEventKind::Renamed) {
                require_reload = true;

                if let Some(previous_path) = event.previous_path.as_ref() {
                    let previous_session = self.edit_session(previous_path)?;
                    if previous_session.should_watch_path(previous_path) {
                        match previous_session.apply(
                            previous_session.head(),
                            previous_path,
                            FileMutation::Removed,
                        ) {
                            Ok(change) => {
                                let updates = self.file_updates(
                                    &previous_session,
                                    change.after,
                                    change.files,
                                )?;

                                result.updates.extend(updates);
                            }
                            Err(error) => result.messages.push(LanguageServiceMessage::warning(
                                "watch_remove_failed",
                                format!(
                                    "watch: failed to remove {}: {error}",
                                    previous_path.display()
                                ),
                            )),
                        }
                    }
                }

                let session = self.edit_session(&event.path)?;
                if session.should_watch_path(&event.path) {
                    match session.read_file_from_fs(&event.path) {
                        Ok(update) => match session.apply(session.head(), &event.path, update) {
                            Ok(change) => {
                                let updates =
                                    self.file_updates(&session, change.after, change.files)?;

                                result.updates.extend(updates);
                            }
                            Err(error) => result.messages.push(LanguageServiceMessage::warning(
                                "watch_update_failed",
                                format!(
                                    "watch: failed to update {}: {error}",
                                    event.path.display()
                                ),
                            )),
                        },
                        Err(error) => result.messages.push(LanguageServiceMessage::warning(
                            "watch_read_failed",
                            format!("watch: failed to read {}: {error}", event.path.display()),
                        )),
                    }
                }

                continue;
            }

            let session = self.edit_session(&event.path)?;
            if !session.should_watch_path(&event.path) {
                continue;
            }

            if matches!(event.kind, FileWatchEventKind::Created) {
                require_reload = true;
            }

            match session.read_file_from_fs(&event.path) {
                Ok(update) => match session.apply(session.head(), &event.path, update) {
                    Ok(change) => {
                        let updates = self.file_updates(&session, change.after, change.files)?;

                        result.updates.extend(updates);
                    }
                    Err(error) => result.messages.push(LanguageServiceMessage::warning(
                        "watch_update_failed",
                        format!("watch: failed to update {}: {error}", event.path.display()),
                    )),
                },
                Err(error) => result.messages.push(LanguageServiceMessage::warning(
                    "watch_read_failed",
                    format!("watch: failed to read {}: {error}", event.path.display()),
                )),
            }
        }

        Ok((result, require_reload))
    }

    /// Reload filesystem state for every root.
    pub fn reload_all(&self) -> Result<LanguageServiceResult, LanguageServiceError> {
        let roots: Vec<PathBuf> = self.roots.iter().map(|entry| entry.key().clone()).collect();

        self.reload_roots(&roots)
    }

    /// Reload filesystem state for specific roots.
    pub fn reload_roots(
        &self,
        roots: &[PathBuf],
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let mut result = LanguageServiceResult::default();

        for root in roots {
            let session = self.session(root)?;
            let change = session.refresh_from_fs(session.head())?;
            let updates = self.file_updates(&session, change.after, change.files)?;

            result.updates.extend(updates);
        }

        Ok(result)
    }

    /// Build one service result for a committed session change.
    fn build_change_result(
        &self,
        path: &Path,
        change: SessionChange,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.edit_session(path)?;
        let repository = session.repository();
        let revision = change.after;
        let Some(module_id) = repository
            .module_id_for_path(revision, path)
            .map_err(LanguageServiceError::from)?
        else {
            let updates = self.file_updates(&session, revision, change.files)?;

            return Ok(LanguageServiceResult::from(updates));
        };
        let profile = repository
            .module_profile(revision, module_id)
            .map_err(LanguageServiceError::from)?;
        let profile_id = profile.id();
        let artifact_keys = [ArtifactKey::dir_checked(module_id, profile_id)];

        session
            .provide(revision, &artifact_keys)
            .map_err(LanguageServiceError::from)?;

        let updates = self.file_updates(&session, revision, change.files)?;
        let diagnostics = self.changed_file_diagnostics(path, &repository)?;
        let updates = self.attach_diagnostics(path, updates, diagnostics);

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

        if let Some(path) = update.file.path.as_deref() {
            update.diagnostic_version =
                session.diagnostic_version(revision, update.file_id, path)?;
        }

        Ok(update)
    }

    /// Return current diagnostics for one direct module path.
    fn changed_file_diagnostics(
        &self,
        path: &Path,
        repository: &Repository,
    ) -> Result<Vec<Diagnostic>, LanguageServiceError> {
        let Some(snapshot) = self.file_diagnostics(path)? else {
            return Ok(Vec::new());
        };

        let file_id = repository.file_id_for_workspace_path(path);
        if snapshot.file.id != file_id {
            return Ok(Vec::new());
        }

        Ok(snapshot.diagnostics)
    }

    /// Attach direct diagnostics to updates for one path.
    fn attach_diagnostics(
        &self,
        path: &Path,
        mut updates: Vec<FileUpdate>,
        diagnostics: Vec<Diagnostic>,
    ) -> Vec<FileUpdate> {
        for update in &mut updates {
            if update.file.path.as_deref() == Some(path) {
                update.diagnostics = diagnostics.clone();
            }
        }

        updates
    }
}
