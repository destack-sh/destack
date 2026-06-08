use destack_artifact::ArtifactKey;
use destack_repository::Revision;

/// One artifact task inside the executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Task {
    /// The pinned repository revision.
    pub(crate) revision: Revision,
    /// The artifact key to provide.
    pub(crate) key: ArtifactKey,
}

impl Task {
    /// Create one artifact task.
    pub(crate) fn new(revision: Revision, key: ArtifactKey) -> Self {
        Self { revision, key }
    }
}
