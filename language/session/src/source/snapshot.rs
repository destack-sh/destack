use std::collections::BTreeMap;

use destack_source::{FileContent, FileContentId, FileId, StringId};
use destack_workspace::{Edit, Repository, RepositoryChange, Revision};

use crate::SessionError;

/// Source that can produce one complete immutable source snapshot.
pub(crate) trait Source {
    /// Read the complete source snapshot visible from this source.
    fn snapshot(&mut self) -> Result<SourceSnapshot, SessionError>;
}

/// One file captured from an external source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceFile {
    /// The stable file identity.
    pub(crate) file_id: FileId,
    /// The interned logical repository path.
    pub(crate) logical_path: StringId,
    /// The exact file content.
    pub(crate) content: FileContent,
}

/// Source truth for one repository revision sync.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceSnapshot {
    /// Files visible in this snapshot by stable file id.
    files: BTreeMap<FileId, SourceFile>,
    /// Whether this snapshot is the complete editable source truth.
    coverage: SourceCoverage,
}

/// The repository edit coverage of one source snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum SourceCoverage {
    /// The snapshot only adds or updates selected files.
    #[default]
    Selected,
    /// The snapshot also removes any editable file missing from it.
    Complete,
}

impl SourceFile {
    /// Create one captured source file.
    pub(crate) fn new(file_id: FileId, logical_path: StringId, content: FileContent) -> Self {
        Self {
            file_id,
            logical_path,
            content,
        }
    }
}

impl SourceSnapshot {
    /// Create one complete source snapshot.
    pub(crate) fn complete() -> Self {
        Self {
            files: BTreeMap::new(),
            coverage: SourceCoverage::Complete,
        }
    }

    /// Create one source snapshot containing one explicit file.
    pub(crate) fn from_file(file: SourceFile) -> Self {
        let mut snapshot = Self::default();
        snapshot.add_file(file);
        snapshot
    }

    /// Add one captured source file.
    pub(crate) fn add_file(&mut self, file: SourceFile) {
        self.files.insert(file.file_id, file);
    }

    /// Return the repository change needed to sync this snapshot.
    pub(crate) fn change(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<RepositoryChange, SessionError> {
        let mut change = RepositoryChange::new();

        // add changed snapshot files
        for (file_id, file) in &self.files {
            let incoming = FileContentId::for_content(&file.content);
            let current = repository.file_content_id(revision, *file_id)?;

            if current != Some(incoming) {
                let logical_path = repository.string_pool().get(file.logical_path).to_string();
                change.push(Edit::SetFile {
                    logical_path,
                    content: file.content.clone(),
                });
            }
        }

        // remove stale editable files for complete snapshots
        if self.coverage == SourceCoverage::Complete {
            for (file_id, logical_path) in repository.editable_file_logical_paths(revision)? {
                if !self.files.contains_key(&file_id) {
                    let logical_path = repository.string_pool().get(logical_path);
                    change.push(Edit::remove_file(logical_path));
                }
            }
        }

        Ok(change)
    }
}
