use std::collections::HashSet;

use destack_source::{FileContent, FileId};
use destack_workspace::{Edit, Repository, RepositoryError, Revision};

use crate::SessionError;

use super::Source;

/// One source sync against one repository revision.
pub(crate) struct SourceSync<'a, S>
where
    S: Source,
{
    /// The repository being synchronized.
    repository: &'a Repository,
    /// The base revision being compared.
    revision: Revision,
    /// The source truth being imported.
    source: &'a mut S,
}

impl<'a, S> SourceSync<'a, S>
where
    S: Source,
{
    /// Create one source sync.
    pub(crate) fn new(repository: &'a Repository, revision: Revision, source: &'a mut S) -> Self {
        Self {
            repository,
            revision,
            source,
        }
    }

    /// Build edits for selected source files.
    pub(crate) fn files(
        &mut self,
        files: impl IntoIterator<Item = S::File>,
    ) -> Result<Vec<Edit>, SessionError> {
        let mut edits = Vec::new();

        // compare selected source files with the repository revision
        for file in files {
            let path = self.source.path(&file);
            let logical_path = path.to_string_lossy().to_string();
            let file_id = FileId::from_logical_path(path);
            let content = self.source.read(&file)?;

            // skip unchanged files
            if !is_changed_file(self.repository, self.revision, file_id, &content)? {
                continue;
            }

            edits.push(Edit::SetFile {
                logical_path,
                content,
            });
        }

        Ok(edits)
    }

    /// Build edits for a complete source sync.
    pub(crate) fn all(&mut self) -> Result<Vec<Edit>, SessionError> {
        // collect visible source files
        let files = self.source.list()?;

        // collect source file edits
        let mut edits = self.files(files)?;

        // collect removals
        self.remove_missing_files(&mut edits)?;

        Ok(edits)
    }

    /// Add removals for revision files no longer present in one source.
    fn remove_missing_files(&mut self, edits: &mut Vec<Edit>) -> Result<(), SessionError> {
        // compare revision files with current source presence
        for file_id in self.repository.file_ids(self.revision)? {
            // skip absent file records and pathless internal records
            let Some(logical_path) = self.repository.file_logical_path(self.revision, file_id)?
            else {
                continue;
            };
            let logical_path = std::path::Path::new(&logical_path);

            // skip files still present in the source
            if !self.source.owns(logical_path)? || self.source.exists(logical_path)? {
                continue;
            }

            edits.push(Edit::remove_file(
                logical_path.to_string_lossy().to_string(),
            ));
        }

        Ok(())
    }
}

/// Return the file ids affected by source edits.
pub(crate) fn edit_file_ids(edits: &[Edit]) -> Vec<FileId> {
    let mut file_ids = Vec::new();
    let mut seen_file_ids = HashSet::new();

    // derive repository file ids from edit paths
    for edit in edits {
        let file_id = edit_file_id(edit);
        if seen_file_ids.insert(file_id) {
            file_ids.push(file_id);
        }
    }

    file_ids
}

/// Apply source edits to one repository revision.
pub(crate) fn apply_edits(
    repository: &Repository,
    revision: Revision,
    edits: Vec<Edit>,
) -> Result<Revision, RepositoryError> {
    if edits.is_empty() {
        return Ok(revision);
    }

    repository.fork_with_edits(revision, edits)
}

/// Return one file id affected by an edit.
fn edit_file_id(edit: &Edit) -> FileId {
    // derive file id from the destination path
    match edit {
        Edit::AddFile { logical_path, .. }
        | Edit::SetFile { logical_path, .. }
        | Edit::RemoveFile { logical_path } => FileId::from_logical_str(logical_path),
        Edit::MoveFile { to, .. } => FileId::from_logical_str(to),
    }
}

/// Return whether one imported file differs from the current revision.
fn is_changed_file(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    content: &FileContent,
) -> Result<bool, RepositoryError> {
    // absent files are always changed
    let Some(file) = repository.file(revision, file_id)? else {
        return Ok(true);
    };

    // compare by payload kind
    let is_changed = match (file.content.payload(), content) {
        (FileContent::Text { content: current }, FileContent::Text { content: incoming }) => {
            current != incoming
        }
        (FileContent::Binary { content: current }, FileContent::Binary { content: incoming }) => {
            current != incoming
        }
        _ => true,
    };

    Ok(is_changed)
}
