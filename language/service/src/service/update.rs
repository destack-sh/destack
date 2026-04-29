use std::collections::HashMap;
use std::path::{Path, PathBuf};

use destack_artifact::ArtifactKey;
use destack_source::{FileWatchEvent, FileWatchEventKind, Uri};
use destack_workspace::Repository;

use super::{
    FileMutation, LanguageService, LanguageServiceError, LanguageServiceMessage,
    LanguageServiceResult, ReloadReason,
};

impl LanguageService {
    /// Set one tracked document to its current editor text.
    pub fn set_document(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        text: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.workspace_for_document_path(path)?;
        let updates = session
            .set_document(path, uri, version, text)
            .map_err(LanguageServiceError::from)?;

        self.realize_direct_module_update(path, updates)
    }

    /// Sync one tracked document to explicit saved text.
    pub fn save_document(
        &self,
        path: &Path,
        text: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.workspace_for_document_path(path)?;
        let updates = session
            .save_document(path, text)
            .map_err(LanguageServiceError::from)?;

        self.realize_direct_module_update(path, updates)
    }

    /// Close one tracked document and restore filesystem backed source truth.
    pub fn close_document(
        &self,
        path: &Path,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.workspace_for_document_path(path)?;
        let updates = session
            .close_document(path)
            .map_err(LanguageServiceError::from)?;

        self.realize_direct_module_update(path, updates)
    }

    /// Apply a virtual file update through the service.
    pub fn update_virtual_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        self.apply_virtual_update(path, FileMutation::Text { content })
    }

    /// Apply an arbitrary virtual file update through the service.
    pub fn apply_virtual_update(
        &self,
        path: &Path,
        update: FileMutation,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.workspace_for_document_path(path)?;
        let updates = session
            .apply_virtual_update(path, update)
            .map_err(LanguageServiceError::from)?;

        self.realize_direct_module_update(path, updates)
    }

    /// Apply watch events through the service.
    pub fn apply_watch_events(
        &self,
        events: Vec<FileWatchEvent>,
        _preferred_uris: HashMap<PathBuf, Uri>,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let (mut result, require_reload) = self.apply_watch_events_immediate(events)?;

        if require_reload {
            let reload = self.reload_all_workspaces(ReloadReason::Update)?;
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
            let is_path_tracked = self.has_tracked_document_for_path(&event.path);
            let is_previous_path_tracked = event
                .previous_path
                .as_ref()
                .is_some_and(|path| self.has_tracked_document_for_path(path));
            if is_path_tracked || is_previous_path_tracked {
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

                let session = self.workspace_for_document_path(&event.path)?;
                if !session.is_watchable_path(&event.path) {
                    continue;
                }

                match session.apply_virtual_update(&event.path, FileMutation::Removed) {
                    Ok(update_result) => {
                        result.updates.extend(update_result);
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
                    let previous_session = self.workspace_for_document_path(previous_path)?;
                    if previous_session.is_watchable_path(previous_path) {
                        match previous_session
                            .apply_virtual_update(previous_path, FileMutation::Removed)
                        {
                            Ok(update_result) => {
                                result.updates.extend(update_result);
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

                let session = self.workspace_for_document_path(&event.path)?;
                if session.is_watchable_path(&event.path) {
                    match session.file_mutation_for_path(&event.path) {
                        Ok(update) => match session.apply_virtual_update(&event.path, update) {
                            Ok(update_result) => {
                                result.updates.extend(update_result);
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

            let session = self.workspace_for_document_path(&event.path)?;
            if !session.is_watchable_path(&event.path) {
                continue;
            }

            if matches!(event.kind, FileWatchEventKind::Created) {
                require_reload = true;
            }

            match session.file_mutation_for_path(&event.path) {
                Ok(update) => match session.apply_virtual_update(&event.path, update) {
                    Ok(update_result) => {
                        result.updates.extend(update_result);
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

    /// Reload tracked filesystem state for every workspace root.
    pub fn reload_all_workspaces(
        &self,
        _reason: ReloadReason,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let roots: Vec<PathBuf> = self
            .workspaces_by_root
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        self.reload_workspaces(&roots)
    }

    /// Reload tracked filesystem state for specific workspace roots.
    pub fn reload_workspaces(
        &self,
        roots: &[PathBuf],
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let mut result = LanguageServiceResult::default();

        for root in roots {
            let session = self.workspace_for_root(root)?;
            let reload = session.reload_filesystem()?;
            result.updates.extend(reload);
        }

        Ok(result)
    }

    /// Realize one directly updated module and attach its diagnostics.
    fn realize_direct_module_update(
        &self,
        path: &Path,
        updates: Vec<destack_session::FileUpdate>,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let session = self.workspace_for_document_path(path)?;
        let repository = session.repository();
        let revision = session.revision();
        let Some(module_id) = repository
            .module_id_for_path(revision, path)
            .map_err(LanguageServiceError::from)?
        else {
            return Ok(LanguageServiceResult::from(updates));
        };
        let profile_id = repository
            .default_profile_id_for_module(revision, module_id)
            .map_err(LanguageServiceError::from)?;
        let artifact_keys = [ArtifactKey::dir_checked(module_id, profile_id)];

        session
            .provide(&artifact_keys)
            .map_err(LanguageServiceError::from)?;

        let diagnostics = self.direct_module_diagnostics(path, &repository)?;
        let updates = self.attach_diagnostics_to_path_updates(path, updates, diagnostics);

        Ok(LanguageServiceResult::from(updates))
    }

    /// Return current diagnostics for one direct module path.
    fn direct_module_diagnostics(
        &self,
        path: &Path,
        repository: &Repository,
    ) -> Result<Vec<destack_source::Diagnostic>, LanguageServiceError> {
        let Some(snapshot) = self.document_diagnostics_for_path(path)? else {
            return Ok(Vec::new());
        };

        let file_id = repository.file_id_for_workspace_path(path);
        if snapshot.file.id != file_id {
            return Ok(Vec::new());
        }

        Ok(snapshot.diagnostics)
    }

    /// Attach direct diagnostics to updates for one path.
    fn attach_diagnostics_to_path_updates(
        &self,
        path: &Path,
        mut updates: Vec<destack_session::FileUpdate>,
        diagnostics: Vec<destack_source::Diagnostic>,
    ) -> Vec<destack_session::FileUpdate> {
        for update in &mut updates {
            if update.file.path.as_deref() == Some(path) {
                update.diagnostics = diagnostics.clone();
            }
        }

        updates
    }
}
