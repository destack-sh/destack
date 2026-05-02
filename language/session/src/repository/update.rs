use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_source::{FileContent, FileId, Uri};
use destack_workspace::{
    ConfigOverride, Edit, Ref, Repository, Revision, apply_config_overrides_to_json,
    parse_jsonc_text,
};
use serde_json::{Map, Value};

use crate::{FileMutation, FileUpdate, FileUpdateKind, RepositoryChange, Session, SessionError};

impl Session {
    /// Apply one explicit file mutation through one ref.
    pub fn apply_file(
        &self,
        reference: &Ref,
        path: &Path,
        update: FileMutation,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let repository = self.repository();
        let before = self.revision(reference)?;
        let change = self.file_mutation_change(repository.as_ref(), path, update);
        let file_ids = change.file_ids();
        let revision = change.apply(repository.as_ref(), before)?;
        let files = self.project_file_updates(before, revision, file_ids)?;

        self.set_ref(reference, revision)?;

        Ok(files)
    }

    /// Build one repository change from one file mutation.
    fn file_mutation_change(
        &self,
        repository: &Repository,
        path: &Path,
        update: FileMutation,
    ) -> RepositoryChange {
        // build the repository edit
        let logical_path = repository.logical_path(path);
        let edit = match update {
            FileMutation::Text { content } => Edit::set_text(logical_path.clone(), content),
            FileMutation::Bytes { content } => Edit::SetFile {
                logical_path: logical_path.clone(),
                content: FileContent::Binary { content },
            },
            FileMutation::Removed => Edit::remove_file(logical_path.clone()),
        };

        RepositoryChange::from_edits([edit])
    }

    /// Project repository file changes into session file updates.
    pub(crate) fn project_file_updates(
        &self,
        before: Revision,
        revision: Revision,
        file_changes: Vec<FileId>,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let mut files = Vec::new();
        let repository = self.repository();

        // project changed repository files into session update payloads
        let mut seen_file_ids = HashSet::new();
        for file_id in file_changes {
            if !seen_file_ids.insert(file_id) {
                continue;
            }

            let file = repository
                .file(revision, file_id)
                .map_err(SessionError::from)?;

            // live files carry their current repository image
            if let Some(file) = file {
                let module_id = repository
                    .module_id_for_file(revision, file_id)
                    .map_err(SessionError::from)?;
                let path = file.path.as_deref().ok_or_else(|| SessionError::Internal {
                    detail: format!("updated repository file has no path: {file_id:?}"),
                })?;
                let kind = FileUpdateKind::for_path(path);

                let uri = Uri::from_file_path(path);

                files.push(FileUpdate::Updated {
                    module_id,
                    file_id,
                    uri,
                    file,
                    kind,
                });

                continue;
            }

            let previous_file = repository
                .file(before, file_id)
                .map_err(SessionError::from)?
                .ok_or(SessionError::FileNotTracked { file_id })?;
            let module_id = repository
                .module_id_for_file(before, file_id)
                .map_err(SessionError::from)?;
            let path = previous_file
                .path
                .as_deref()
                .ok_or_else(|| SessionError::Internal {
                    detail: format!("removed repository file has no path: {file_id:?}"),
                })?;
            let kind = FileUpdateKind::for_path(path);

            // removed files still need a uri so clients can clear diagnostics
            let uri = Uri::from_file_path(path);

            files.push(FileUpdate::Removed {
                module_id,
                file_id,
                uri,
                kind,
            });
        }

        Ok(files)
    }

    /// Apply workspace config overrides through one coherent session mutation.
    pub fn apply_workspace_config_overrides(
        &self,
        reference: &Ref,
        overrides: &[ConfigOverride],
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();

        if overrides.is_empty() {
            return Ok(Vec::new());
        }

        // revision fork
        let repository = self.repository();
        let before = self.revision(reference)?;
        let config_paths = self.collect_workspace_config_paths(repository.as_ref(), before)?;

        // config updates
        let mut change = RepositoryChange::new();
        let mut file_changes = Vec::new();
        let mut seen_file_ids = HashSet::new();

        for path in config_paths {
            let mut json = self.load_workspace_config_json(repository.as_ref(), before, &path)?;
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

            let file_change = self.file_mutation_change(
                repository.as_ref(),
                &path,
                FileMutation::Text { content },
            );

            for file_id in file_change.file_ids() {
                if !seen_file_ids.insert(file_id) {
                    continue;
                }

                file_changes.push(file_id);
            }

            change.extend(file_change);
        }

        let revision = change.apply(repository.as_ref(), before)?;
        let files = self.project_file_updates(before, revision, file_changes)?;

        self.set_ref(reference, revision)?;

        Ok(files)
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
        let file_id = repository.file_id(path);

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
