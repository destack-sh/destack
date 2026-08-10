use destack_core::{Blob, TreapRoot};
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::repository::Repository;

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
    /// Compare one file binding between two repository file trees.
    pub(crate) fn between(
        repository: &Repository,
        before: TreapRoot,
        after: TreapRoot,
        file: FileId,
    ) -> Option<Self> {
        let before = repository.files.entries.get(before, &file);
        let after = repository.files.entries.get(after, &file);
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
