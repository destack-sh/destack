use std::collections::HashSet;
use std::path::Path;

use destack_source::{FileContent, FileId, Uri};
use destack_workspace::{Edit, Ref, Repository, RepositoryChange, Revision};

use crate::{FileChange, FileUpdate, FileUpdateKind, Session, SessionError};

impl Session {
    /// Apply one explicit file change through one ref.
    pub fn apply_file(
        &self,
        reference: &Ref,
        path: &Path,
        update: FileChange,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let repository = self.repository();
        let edit = Self::edit_for_file(repository.as_ref(), path, update);
        let change = RepositoryChange::from_edit(edit);
        let file_ids = change.file_ids().to_vec();
        let before = self.revision(reference)?;
        let revision = repository.commit_change(before, change)?;
        let _revision_pin = repository.pin(revision)?;

        // publish when the ref still points at the edited base
        let was_published = repository.advance_ref(reference, before, revision)?;
        if !was_published {
            return Err(SessionError::StaleRevision {
                reference: reference.clone(),
                expected: before,
                current: self.revision(reference)?,
            });
        }

        self.project_file_updates(before, revision, file_ids)
    }

    /// Build source edits from one file change.
    fn edit_for_file(repository: &Repository, path: &Path, update: FileChange) -> Edit {
        let logical_path = repository.logical_path(path);
        match update {
            FileChange::Text { content } => Edit::set_text(logical_path, content),
            FileChange::Bytes { content } => Edit::SetFile {
                logical_path,
                content: FileContent::Binary { content },
            },
            FileChange::Removed => Edit::remove_file(logical_path),
        }
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
