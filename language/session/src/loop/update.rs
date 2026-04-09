use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_source::{Diagnostic, FileContent, FileId, Uri};
use destack_workspace::{
    Change, ConfigOverride, Edit, Repository, Revision, apply_config_overrides_to_json,
    parse_jsonc_text,
};
use serde_json::{Map, Value};

use crate::session::FileChange;
use crate::{FileChangeKind, FileMutation, FileUpdate, Session, SessionError};

impl Session {
    /// Set one tracked file to its current editor text.
    pub fn set_document(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        text: String,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();

        // reject stale editor updates at the session boundary
        if let Some((_, current_version)) = self.open_file_identity_for_path(path)
            && version <= current_version
        {
            return Err(SessionError::StaleOpenFileVersion {
                path: path.to_path_buf(),
                incoming: version,
                current: current_version,
            });
        }

        let update = FileMutation::Text {
            content: text.clone(),
        };
        let (revision, file_changes) = self.apply_file_update_locked(path, update)?;

        self.set_open_file(path, uri.clone(), version);
        self.set_overlay_for_path(path, text);

        self.publish_file_update(revision, file_changes, Default::default(), Some(&uri))
    }

    /// Save one tracked file to explicit saved text.
    pub fn save_document(
        &self,
        path: &Path,
        text: String,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();

        // preserve tracked editor identity for still open files
        let tracked_document = self.open_file_identity_for_path(path);
        let update = FileMutation::Text {
            content: text.clone(),
        };
        let (revision, file_changes) = self.apply_file_update_locked(path, update)?;

        if let Some((uri, version)) = tracked_document {
            self.set_open_file(path, uri.clone(), version);
            self.set_overlay_for_path(path, text);

            return self.publish_file_update(
                revision,
                file_changes,
                Default::default(),
                Some(&uri),
            );
        }

        self.publish_file_update(revision, file_changes, Default::default(), None)
    }

    /// Close one tracked file and restore filesystem backed source truth.
    pub fn close_document(&self, path: &Path) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();

        let Some((uri, version)) = self.open_file_identity_for_path(path) else {
            return Ok(Vec::new());
        };
        let tracked_text =
            self.open_file_text_for_path(path)
                .map_err(|error| SessionError::ReadPathFailed {
                    detail: format!(
                        "failed to read tracked open document {}: {error}",
                        path.display(),
                    ),
                    path: path.to_path_buf(),
                })?;

        let _closed_document = self.close_open_file(path);
        self.remove_overlay_for_path(path);

        // restore the filesystem backed update
        let update = match self.file_mutation_for_path(path) {
            Ok(update) => update,
            Err(error) if error.kind() == ErrorKind::NotFound => FileMutation::Removed,
            Err(error) => {
                self.set_open_file(path, uri.clone(), version);
                if let Some(text) = tracked_text.clone() {
                    self.set_overlay_for_path(path, text);
                }

                return Err(SessionError::ReadPathFailed {
                    detail: format!(
                        "failed to restore closed document {}: {error}",
                        path.display(),
                    ),
                    path: path.to_path_buf(),
                });
            }
        };

        // restore the open document state when the semantic update fails
        let (revision, file_changes) = match self.apply_file_update_locked(path, update) {
            Ok(update) => update,
            Err(error) => {
                self.set_open_file(path, uri.clone(), version);
                if let Some(text) = tracked_text {
                    self.set_overlay_for_path(path, text);
                }

                return Err(error);
            }
        };

        self.publish_file_update(revision, file_changes, Default::default(), Some(&uri))
    }

    /// Apply an arbitrary virtual file update through the session.
    pub fn apply_virtual_update(
        &self,
        path: &Path,
        update: FileMutation,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let (revision, file_changes) = self.apply_file_update_locked(path, update)?;

        self.publish_file_update(revision, file_changes, Default::default(), None)
    }

    /// Build one file change after one file update.
    pub(super) fn file_change(
        &self,
        repository: &Repository,
        revision: Revision,
        path: &Path,
        file_id: FileId,
    ) -> Result<FileChange, SessionError> {
        let module_id = repository
            .module_id_for_file(revision, file_id)
            .map_err(SessionError::from)?;

        Ok(FileChange {
            module_id,
            file_id,
            kind: FileChangeKind::for_path(path),
        })
    }

    /// Apply one file update to one staged revision.
    pub(super) fn apply_file_update_to_revision(
        &self,
        repository: &Repository,
        revision: Revision,
        path: &Path,
        update: FileMutation,
    ) -> Result<Revision, SessionError> {
        // apply the explicit file edit
        let logical_path = repository.normalize_workspace_path(path);
        let edit = match update {
            FileMutation::Text { content } => Edit::set_text(logical_path.clone(), content),
            FileMutation::Bytes { content } => Edit::SetFile {
                logical_path: logical_path.clone(),
                content: FileContent::Binary { content },
            },
            FileMutation::Removed => Edit::remove_file(logical_path.clone()),
        };
        let change = Change::from(vec![edit]);

        self.apply_source_change(repository, revision, change)
    }

    /// Apply one staged source change directly.
    pub(super) fn apply_source_change(
        &self,
        repository: &Repository,
        revision: Revision,
        change: Change,
    ) -> Result<Revision, SessionError> {
        repository
            .apply_to_revision(revision, change)
            .map_err(SessionError::from)
    }

    /// Apply one batch of source-side file requirements to a staged revision.
    pub(super) fn apply_file_requirements(
        &self,
        repository: &Repository,
        revision: Revision,
        requirement: &destack_workspace::RequirementSet,
    ) -> Result<Revision, SessionError> {
        let revision = repository
            .fork_mutable_revision(revision)
            .map_err(SessionError::from)?;
        let mut change = Change::empty();

        // source changes
        requirement.for_each_source(|requirement| {
            change.extend_from(&requirement.change);
        });

        let revision = self.apply_source_change(repository, revision, change)?;

        repository
            .freeze_revision(revision)
            .map_err(SessionError::from)
    }

    /// Apply one file update while already holding the session mutation lock.
    fn apply_file_update_locked(
        &self,
        path: &Path,
        update: FileMutation,
    ) -> Result<(Revision, Vec<FileChange>), SessionError> {
        let repository = self.repository();
        let revision = self.ensure_mutable_revision()?;
        let file_id = repository.file_id_for_workspace_path(path);
        let is_module_update =
            !matches!(&update, FileMutation::Removed) && self.is_scannable_module_path(path);

        // apply the explicit file edit
        let mut revision =
            self.apply_file_update_to_revision(&repository, revision, path, update)?;

        // admit newly visible module files after the edit lands
        if is_module_update && self.existing_module_id_for_path(revision, path)?.is_none() {
            let (next_revision, _) =
                self.admit_module_for_revision(revision, path)
                    .map_err(|error| SessionError::UpdatePathFailed {
                        path: path.to_path_buf(),
                        detail: format!("failed to discover session module: {error}"),
                    })?;
            revision = next_revision;
        }

        let file_change = self.file_change(&repository, revision, path, file_id)?;
        let file_changes = vec![file_change];

        Ok((revision, file_changes))
    }

    /// Publish one finished file update.
    pub(super) fn publish_file_update(
        &self,
        revision: Revision,
        file_changes: Vec<FileChange>,
        diagnostics_by_file: HashMap<FileId, Vec<Diagnostic>>,
        preferred_uri: Option<&Uri>,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        self.publish_revision(revision)?;

        self.file_updates(revision, file_changes, diagnostics_by_file, preferred_uri)
    }

    /// Apply workspace config overrides through one coherent session mutation.
    pub fn apply_workspace_config_overrides(
        &self,
        overrides: &[ConfigOverride],
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();

        if overrides.is_empty() {
            return Ok(Vec::new());
        }

        // staged revision
        let repository = self.repository();
        let mut revision = self.ensure_mutable_revision()?;
        let config_paths = self.collect_workspace_config_paths(repository.as_ref(), revision)?;

        // staged config updates
        let mut file_changes = Vec::new();
        let mut seen_file_ids = HashSet::new();

        for path in config_paths {
            let mut json = self.load_workspace_config_json(repository.as_ref(), revision, &path)?;
            apply_config_overrides_to_json(&mut json, overrides).map_err(|detail| {
                SessionError::UpdatePathFailed {
                    path: path.clone(),
                    detail,
                }
            })?;

            let content = serde_json::to_string_pretty(&json).map_err(|error| {
                SessionError::UpdatePathFailed {
                    path: path.clone(),
                    detail: format!("failed to serialize {}: {error}", path.display()),
                }
            })?;
            let content = format!("{content}\n");

            revision = self.apply_file_update_to_revision(
                repository.as_ref(),
                revision,
                &path,
                FileMutation::Text { content },
            )?;

            let file_id = repository.file_id_for_workspace_path(&path);
            if seen_file_ids.insert(file_id) {
                let file_change =
                    self.file_change(repository.as_ref(), revision, &path, file_id)?;
                file_changes.push(file_change);
            }
        }

        self.publish_file_update(revision, file_changes, Default::default(), None)
    }

    /// Collect visible workspace config paths for the current revision.
    fn collect_workspace_config_paths(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<Vec<PathBuf>, SessionError> {
        let mut paths = vec![self.root().join("destack.json")];

        for package_path in repository
            .workspace_package_paths(revision)
            .map_err(SessionError::from)?
        {
            paths.push(package_path.join("destack.json"));
        }

        paths.sort();
        paths.dedup();

        Ok(paths)
    }

    /// Load one config json value from the current revision.
    fn load_workspace_config_json(
        &self,
        repository: &Repository,
        revision: Revision,
        path: &Path,
    ) -> Result<Value, SessionError> {
        let file_id = repository.file_id_for_workspace_path(path);

        // prefer revision backed source truth
        if let Some(file) = repository
            .file(revision, file_id)
            .map_err(SessionError::from)?
        {
            return parse_jsonc_text(file.text()).map_err(|error| SessionError::UpdatePathFailed {
                path: path.to_path_buf(),
                detail: format!("failed to parse {}: {error}", path.display()),
            });
        }

        // fall back to physical source when the config file is not tracked yet
        if let Ok(content) = repository.file_system().read_to_string(path) {
            return parse_jsonc_text(&content).map_err(|error| SessionError::UpdatePathFailed {
                path: path.to_path_buf(),
                detail: format!("failed to parse {}: {error}", path.display()),
            });
        }

        Ok(Value::Object(Map::new()))
    }
}
