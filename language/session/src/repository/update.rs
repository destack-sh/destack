use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_source::{Diagnostic, FileContent, FileId, Uri};
use destack_workspace::{
    ConfigOverride, Edit, Ref, Repository, Revision, apply_config_overrides_to_json,
    parse_jsonc_text,
};
use serde_json::{Map, Value};

use crate::session::{FileChange, file_update_image_from_file};
use crate::{FileChangeKind, FileMutation, FileUpdate, Session, SessionChange, SessionError};

impl Session {
    /// Set one open file to its current editor text.
    pub fn update_open_file(
        &self,
        reference: &Ref,
        path: &Path,
        uri: Uri,
        version: i32,
        text: String,
    ) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();

        // reject stale editor updates at the session boundary
        if let Some((_, current_version)) = self.open_file_identity(path)
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
        let before = self.revision(reference)?;
        let (after, file_changes) = self.apply_file_update_locked(before, path, update)?;

        self.track_open_file(path, uri.clone(), version);
        self.set_overlay_for_path(path, text);

        self.advance(
            reference,
            before,
            after,
            file_changes,
            HashMap::new(),
            Some(&uri),
        )
    }

    /// Save one open file to explicit saved text.
    pub fn save_open_file(
        &self,
        reference: &Ref,
        path: &Path,
        text: String,
    ) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();

        // preserve tracked editor identity for still open files
        let tracked_open_file = self.open_file_identity(path);
        let update = FileMutation::Text {
            content: text.clone(),
        };
        let before = self.revision(reference)?;
        let (after, file_changes) = self.apply_file_update_locked(before, path, update)?;

        if let Some((uri, version)) = tracked_open_file {
            self.track_open_file(path, uri.clone(), version);
            self.set_overlay_for_path(path, text);

            return self.advance(
                reference,
                before,
                after,
                file_changes,
                HashMap::new(),
                Some(&uri),
            );
        }

        self.advance(reference, before, after, file_changes, HashMap::new(), None)
    }

    /// Close one open file and restore filesystem backed source truth.
    pub fn close_open_file(
        &self,
        reference: &Ref,
        path: &Path,
    ) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();

        let Some((uri, version)) = self.open_file_identity(path) else {
            let revision = self.revision(reference)?;

            return Ok(SessionChange {
                reference: reference.clone(),
                before: revision,
                after: revision,
                files: Vec::new(),
            });
        };
        let tracked_text =
            self.open_file_text(path)
                .map_err(|error| SessionError::ReadPathFailed {
                    detail: format!(
                        "failed to read tracked open file {}: {error}",
                        path.display()
                    ),
                    path: path.to_path_buf(),
                })?;

        let _removed_open_file = self.untrack_open_file(path);
        self.remove_overlay_for_path(path);

        // restore the filesystem backed update
        let update = match self.file_mutation_for_path(path) {
            Ok(update) => update,
            Err(error) if error.kind() == ErrorKind::NotFound => FileMutation::Removed,
            Err(error) => {
                self.track_open_file(path, uri.clone(), version);
                if let Some(text) = tracked_text.clone() {
                    self.set_overlay_for_path(path, text);
                }

                return Err(SessionError::ReadPathFailed {
                    detail: format!(
                        "failed to restore closed open file {}: {error}",
                        path.display(),
                    ),
                    path: path.to_path_buf(),
                });
            }
        };

        // restore the open file state when the semantic update fails
        let before = self.revision(reference)?;
        let (after, file_changes) = match self.apply_file_update_locked(before, path, update) {
            Ok(update) => update,
            Err(error) => {
                self.track_open_file(path, uri.clone(), version);
                if let Some(text) = tracked_text {
                    self.set_overlay_for_path(path, text);
                }

                return Err(error);
            }
        };

        self.advance(
            reference,
            before,
            after,
            file_changes,
            HashMap::new(),
            Some(&uri),
        )
    }

    /// Apply one explicit file mutation through one ref.
    pub fn apply(
        &self,
        reference: &Ref,
        path: &Path,
        update: FileMutation,
    ) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let before = self.revision(reference)?;
        let (after, file_changes) = self.apply_file_update_locked(before, path, update)?;

        self.advance(reference, before, after, file_changes, HashMap::new(), None)
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
        self.apply_edits(repository, revision, [edit])
    }

    /// Apply staged repository edits directly.
    pub(super) fn apply_edits<I>(
        &self,
        repository: &Repository,
        revision: Revision,
        edits: I,
    ) -> Result<Revision, SessionError>
    where
        I: IntoIterator<Item = Edit>,
    {
        repository
            .fork_with_edits(revision, edits)
            .map_err(SessionError::from)
    }

    /// Apply one file update while already holding the session mutation lock.
    fn apply_file_update_locked(
        &self,
        revision: Revision,
        path: &Path,
        update: FileMutation,
    ) -> Result<(Revision, Vec<FileChange>), SessionError> {
        let repository = self.repository();
        let file_id = repository.file_id_for_workspace_path(path);
        let is_module_update =
            !matches!(&update, FileMutation::Removed) && self.is_scannable_module_path(path);

        // apply the explicit file edit
        let mut revision =
            self.apply_file_update_to_revision(&repository, revision, path, update)?;
        let module_id = repository
            .module_id_for_path(revision, path)
            .map_err(SessionError::from)?;

        // load newly visible module files after the edit lands
        if is_module_update && module_id.is_none() {
            let (next_revision, _) =
                self.load_module_revision(revision, path).map_err(|error| {
                    SessionError::UpdatePathFailed {
                        path: path.to_path_buf(),
                        detail: format!("failed to load session module: {error}"),
                    }
                })?;
            revision = next_revision;
        }

        let file_change = self.file_change(&repository, revision, path, file_id)?;
        let file_changes = vec![file_change];

        Ok((revision, file_changes))
    }

    /// Advance one ref and build its visible file update payload.
    pub(super) fn advance(
        &self,
        reference: &Ref,
        before: Revision,
        after: Revision,
        file_changes: Vec<FileChange>,
        mut diagnostics_by_file: HashMap<FileId, Vec<Diagnostic>>,
        preferred_uri: Option<&Uri>,
    ) -> Result<SessionChange, SessionError> {
        self.commit_revision(reference, after)?;

        let mut files = Vec::new();

        // project changed repository files into session update payloads
        for file_change in file_changes {
            let file = self
                .repository()
                .file(after, file_change.file_id)
                .map_err(SessionError::from)?
                .ok_or(SessionError::FileIdNotTracked {
                    file_id: file_change.file_id,
                })?;
            let file = file_update_image_from_file(&file);
            let diagnostics = diagnostics_by_file
                .remove(&file_change.file_id)
                .unwrap_or_default();

            // prefer live editor identity for diagnostic publishing
            let open_file_identity = file
                .path
                .as_ref()
                .and_then(|path| self.open_file_identity(path));
            let diagnostic_uri = open_file_identity
                .as_ref()
                .map(|(uri, _)| uri.clone())
                .or_else(|| preferred_uri.cloned())
                .or_else(|| file.path.as_ref().map(Uri::from_file_path))
                .unwrap_or_else(|| file.uri.clone());
            let diagnostic_version = open_file_identity.as_ref().map(|(_, version)| *version);

            files.push(FileUpdate {
                module_id: file_change.module_id,
                file_id: file_change.file_id,
                diagnostic_uri,
                diagnostic_version,
                file,
                kind: file_change.kind,
                diagnostics,
            });
        }

        Ok(SessionChange {
            reference: reference.clone(),
            before,
            after,
            files,
        })
    }

    /// Apply workspace config overrides through one coherent session mutation.
    pub fn apply_workspace_config_overrides(
        &self,
        reference: &Ref,
        overrides: &[ConfigOverride],
    ) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();

        if overrides.is_empty() {
            let revision = self.revision(reference)?;

            return Ok(SessionChange {
                reference: reference.clone(),
                before: revision,
                after: revision,
                files: Vec::new(),
            });
        }

        // staged revision
        let repository = self.repository();
        let before = self.revision(reference)?;
        let mut revision = before;
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

        self.advance(
            reference,
            before,
            revision,
            file_changes,
            HashMap::new(),
            None,
        )
    }

    /// Collect visible workspace config paths for the current revision.
    fn collect_workspace_config_paths(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<Vec<PathBuf>, SessionError> {
        let mut paths = vec![self.root().join("destack.json")];

        for package_path in repository
            .package_roots(revision)
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

        // read physical source when the config file is not tracked yet
        let content = match repository.file_system().read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Value::Object(Map::new()));
            }
            Err(error) => {
                return Err(SessionError::ReadPathFailed {
                    path: path.to_path_buf(),
                    detail: format!("failed to read {}: {error}", path.display()),
                });
            }
        };

        parse_jsonc_text(&content).map_err(|error| SessionError::UpdatePathFailed {
            path: path.to_path_buf(),
            detail: format!("failed to parse {}: {error}", path.display()),
        })
    }
}
