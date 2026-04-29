use destack_artifact::ArtifactKey;
use destack_workspace::Revision;

/// One artifact task inside a session loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SessionTask {
    /// The pinned repository revision.
    pub(crate) revision: Revision,
    /// The artifact key to provide.
    pub(crate) key: ArtifactKey,
}

impl SessionTask {
    /// Create one artifact task.
    pub(crate) fn new(revision: Revision, key: ArtifactKey) -> Self {
        Self { revision, key }
    }
}
