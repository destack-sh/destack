use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_source::{FileContent, FileId, Uri};
use destack_workspace::{
    ConfigOverride, Edit, Ref, Repository, Revision, apply_config_overrides_to_json,
    parse_jsonc_text,
};
use serde_json::{Map, Value};

use crate::session::FileChange;
use crate::{FileChangeKind, FileMutation, FileUpdate, Session, SessionChange, SessionError};

impl Session {
    /// Set one open file to its current overlay text.
    pub fn update_open_file(
        &self,
        reference: &Ref,
        path: &Path,
        uri: Uri,
        text: String,
    ) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();

        let update = FileMutation::Text {
            content: text.clone(),
        };
        let before = self.revision(reference)?;
        let (after, file_changes) = self.apply_file_update_locked(before, path, update)?;

        self.track_open_file(path, uri.clone());
        self.set_open_file_text(path, text);

        self.finish_change(reference, before, after, file_changes, Some(&uri))
    }

    /// Save one open file to explicit saved text.
    pub fn save_open_file(
        &self,
        reference: &Ref,
        path: &Path,
        text: String,
    ) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();

        // preserve open file state for still open files
        let tracked_open_file = self.tracked_open_file(path);
        let update = FileMutation::Text {
            content: text.clone(),
        };
        let before = self.revision(reference)?;
        let (after, file_changes) = self.apply_file_update_locked(before, path, update)?;

        if let Some(open_file) = tracked_open_file {
            self.set_open_file_text(path, text);

            return self.finish_change(
                reference,
                before,
                after,
                file_changes,
                Some(&open_file.uri),
            );
        }

        self.finish_change(reference, before, after, file_changes, None)
    }

    /// Close one open file and restore filesystem backed source truth.
    pub fn close_open_file(
        &self,
        reference: &Ref,
        path: &Path,
    ) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();

        let Some(open_file) = self.tracked_open_file(path) else {
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
        self.remove_open_file_text(path);

        // restore the filesystem backed update
        let update = match self.read_file_from_fs(path) {
            Ok(update) => update,
            Err(error) if error.kind() == ErrorKind::NotFound => FileMutation::Removed,
            Err(error) => {
                self.track_open_file(path, open_file.uri.clone());
                if let Some(text) = tracked_text.clone() {
                    self.set_open_file_text(path, text);
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
                self.track_open_file(path, open_file.uri.clone());
                if let Some(text) = tracked_text {
                    self.set_open_file_text(path, text);
                }

                return Err(error);
            }
        };

        self.finish_change(reference, before, after, file_changes, Some(&open_file.uri))
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

        self.finish_change(reference, before, after, file_changes, None)
    }

    /// Build one file change after one file update.
    pub(crate) fn file_change(
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
    pub(crate) fn apply_file_update_to_revision(
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
    pub(crate) fn apply_edits<I>(
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
            !matches!(&update, FileMutation::Removed) && self.is_import_module_path(path);

        // apply the explicit file edit
        let mut revision =
            self.apply_file_update_to_revision(&repository, revision, path, update)?;
        let module_id = repository
            .module_id_for_path(revision, path)
            .map_err(SessionError::from)?;

        // load newly visible module files after the edit lands
        if is_module_update && module_id.is_none() {
            let (next_revision, _) = self.import_module_file(revision, path).map_err(|error| {
                SessionError::UpdatePathFailed {
                    path: path.to_path_buf(),
                    detail: format!("failed to import session module: {error}"),
                }
            })?;
            revision = next_revision;
        }

        let file_change = self.file_change(&repository, revision, path, file_id)?;
        let file_changes = vec![file_change];

        Ok((revision, file_changes))
    }

    /// Finish one session change by setting the ref and building file updates.
    pub(crate) fn finish_change(
        &self,
        reference: &Ref,
        before: Revision,
        after: Revision,
        file_changes: Vec<FileChange>,
        preferred_uri: Option<&Uri>,
    ) -> Result<SessionChange, SessionError> {
        self.set_ref(reference, after)?;

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

            // prefer live open file uri for diagnostic publishing
            let open_file = file
                .path
                .as_ref()
                .and_then(|path| self.tracked_open_file(path));
            let uri = open_file
                .as_ref()
                .map(|file| file.uri.clone())
                .or_else(|| preferred_uri.cloned())
                .or_else(|| file.path.as_ref().map(Uri::from_file_path))
                .unwrap_or_else(|| file.uri.clone());

            files.push(FileUpdate {
                module_id: file_change.module_id,
                file_id: file_change.file_id,
                uri,
                file,
                kind: file_change.kind,
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

        self.finish_change(reference, before, revision, file_changes, None)
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
