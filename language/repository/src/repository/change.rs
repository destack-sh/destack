use serde::{Deserialize, Serialize};
use tspp_core::Blob;
use tspp_serde::Reflect;
use tspp_source::FileId;

use crate::repository::{Edit, FileEntry, Repository, RepositoryError, Revision};

/// One canonical file difference between two repository revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Change {
    /// The path-derived file identity.
    pub file: FileId,
    /// The normalized repository-relative path.
    pub path: String,
    /// Blob in the previous revision.
    pub before: Option<Blob>,
    /// Blob in the updated revision.
    pub after: Option<Blob>,
}

impl Change {
    /// Build the repository edit that applies this change.
    pub fn forward(&self) -> Edit {
        match self.after {
            Some(blob) => Edit::set_file(&self.path, blob),
            None => Edit::remove_file(&self.path),
        }
    }

    /// Compare one file binding between two repository file trees.
    pub(crate) fn between(
        repository: &Repository,
        file: FileId,
        before: Option<FileEntry>,
        after: Option<FileEntry>,
    ) -> Option<Self> {
        if before == after {
            return None;
        }

        // resolve the stable logical path from either surviving binding
        let entry = after.as_ref().or(before.as_ref())?;
        let path = repository.logical_path_text(entry.logical_path).to_string();

        Some(Self {
            file,
            path,
            before: before.map(|entry| entry.blob),
            after: after.map(|entry| entry.blob),
        })
    }
}

impl Repository {
    /// Compare every file binding between two immutable revisions.
    pub fn changes(
        &self,
        before: Revision,
        after: Revision,
    ) -> Result<Vec<Change>, RepositoryError> {
        let before = self.file_entries(before)?;
        let after = self.file_entries(after)?;
        let mut before_index = 0;
        let mut after_index = 0;
        let mut changes = Vec::new();

        // merge both key ordered file arrays without rebuilding an index
        loop {
            let before_file = before.get(before_index).copied();
            let after_file = after.get(after_index).copied();

            let (file, before_entry, after_entry) = match (before_file, after_file) {
                // compare two bindings for the same file
                (Some((before_file, before_entry)), Some((after_file, after_entry)))
                    if before_file == after_file =>
                {
                    before_index += 1;
                    after_index += 1;

                    (before_file, Some(before_entry), Some(after_entry))
                }
                // consume one file removed from the updated revision
                (Some((before_file, before_entry)), Some((after_file, _)))
                    if before_file < after_file =>
                {
                    before_index += 1;

                    (before_file, Some(before_entry), None)
                }
                // consume one file added to the updated revision
                (Some(_), Some((after_file, after_entry))) => {
                    after_index += 1;

                    (after_file, None, Some(after_entry))
                }
                // drain files remaining only in the previous revision
                (Some((before_file, before_entry)), None) => {
                    before_index += 1;

                    (before_file, Some(before_entry), None)
                }
                // drain files remaining only in the updated revision
                (None, Some((after_file, after_entry))) => {
                    after_index += 1;

                    (after_file, None, Some(after_entry))
                }
                // finish after both arrays are exhausted
                (None, None) => break,
            };

            // retain changed bindings
            if let Some(change) = Change::between(self, file, before_entry, after_entry) {
                changes.push(change);
            }
        }

        // present changes in stable logical path order
        changes.sort_unstable_by(|left, right| left.path.cmp(&right.path));

        Ok(changes)
    }
}
