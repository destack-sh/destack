use std::collections::HashSet;
use std::path::Path;

use destack_source::{FileContent, FileId, Uri};
use destack_workspace::{Edit, Ref, Repository, Revision};

use crate::{FileChange, FileUpdate, FileUpdateKind, RepositoryChange, Session, SessionError};

impl Session {
    /// Apply one explicit file change through one ref.
    pub fn apply_file(
        &self,
        reference: &Ref,
        path: &Path,
        update: FileChange,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let repository = self.repository();
        let before = self.revision(reference)?;
        let change = self.repository_change_for_file(repository.as_ref(), path, update);
        let file_ids = change.file_ids();
        let revision = change.apply(repository.as_ref(), before)?;
        let files = self.project_file_updates(before, revision, file_ids)?;

        self.set_ref(reference, revision)?;

        Ok(files)
    }

    /// Build one repository change from one file change.
    fn repository_change_for_file(
        &self,
        repository: &Repository,
        path: &Path,
        update: FileChange,
    ) -> RepositoryChange {
        // build the repository edit
        let logical_path = repository.logical_path(path);
        let edit = match update {
            FileChange::Text { content } => Edit::set_text(logical_path, content),
            FileChange::Bytes { content } => Edit::SetFile {
                logical_path,
                content: FileContent::Binary { content },
            },
            FileChange::Removed => Edit::remove_file(logical_path),
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
}
