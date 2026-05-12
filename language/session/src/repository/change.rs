use std::collections::HashSet;

use destack_source::{FileContent, FileId};
use destack_workspace::{Edit, Repository, RepositoryError, Revision};

use super::RepositorySource;

/// One repository change derived from external source truth.
#[derive(Debug, Default)]
pub(crate) struct RepositoryChange {
    /// The edits that make the repository match external source truth.
    edits: Vec<Edit>,
}

impl RepositoryChange {
    /// Create one repository change from explicit edits.
    pub(crate) fn from_edits(edits: impl IntoIterator<Item = Edit>) -> Self {
        Self {
            edits: edits.into_iter().collect(),
        }
    }

    /// Create one repository change from selected source files.
    pub(crate) fn from_files<S>(
        repository: &Repository,
        revision: Revision,
        source: &S,
        files: impl IntoIterator<Item = S::File>,
    ) -> Result<Self, RepositoryError>
    where
        S: RepositorySource,
    {
        let mut edits = Vec::new();

        // compare selected source files with the repository revision
        for file in files {
            let path = source.path(&file);
            let file_id = FileId::from_logical_path(path);
            let content = source.read(&file)?;

            // skip unchanged files
            if !is_changed_file(repository, revision, file_id, &content)? {
                continue;
            }

            edits.push(Edit::SetFile {
                logical_path: path.to_string_lossy().to_string(),
                content,
            });
        }

        Ok(Self { edits })
    }

    /// Create one repository change from a complete source sync.
    pub(crate) fn from_source<S>(
        repository: &Repository,
        revision: Revision,
        source: &mut S,
    ) -> Result<Self, RepositoryError>
    where
        S: RepositorySource,
    {
        // collect visible source files
        let files = source.list()?;

        // collect file edits
        let mut change = Self::from_files(repository, revision, source, files)?;

        // collect removals
        change.remove_missing_files(repository, revision, source)?;

        Ok(change)
    }

    /// Return whether this change has no edits.
    pub(crate) fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Add removals for tracked files no longer present in one source.
    fn remove_missing_files<S>(
        &mut self,
        repository: &Repository,
        revision: Revision,
        source: &S,
    ) -> Result<(), RepositoryError>
    where
        S: RepositorySource,
    {
        // compare tracked files with current source presence
        for file_id in repository.file_ids(revision)? {
            // skip absent file records
            let Some(file) = repository.file(revision, file_id)? else {
                continue;
            };

            // skip files without source paths
            let Some(path) = file.path.as_ref() else {
                continue;
            };

            // skip files still owned by the source
            if !source.tracks(path) || source.has(path)? {
                continue;
            }

            let logical_path = repository.logical_path(path);
            self.edits.push(Edit::remove_file(logical_path));
        }

        Ok(())
    }

    /// Return the file ids affected by this change.
    pub(crate) fn file_ids(&self) -> Vec<FileId> {
        let mut file_ids = Vec::new();
        let mut seen_file_ids = HashSet::new();

        // derive repository file ids from edit paths
        for edit in &self.edits {
            let file_id = edit_file_id(edit);
            if seen_file_ids.insert(file_id) {
                file_ids.push(file_id);
            }
        }

        file_ids
    }

    /// Apply this change to one repository revision.
    pub(crate) fn apply(
        self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<Revision, RepositoryError> {
        // empty changes preserve the current revision
        if self.is_empty() {
            return Ok(revision);
        }

        repository.fork_with_edits(revision, self.edits)
    }
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
