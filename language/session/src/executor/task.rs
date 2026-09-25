use tspp_artifact::ArtifactKey;
use tspp_repository::Revision;

/// Identity of one repository-specific artifact session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SessionId(pub(crate) u32);

/// One artifact task inside the executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Task {
    /// The session that owns this task.
    pub(crate) session: SessionId,
    /// The pinned repository revision.
    pub(crate) revision: Revision,
    /// The artifact key to provide.
    pub(crate) key: ArtifactKey,
}

impl Task {
    /// Create one artifact task.
    pub(crate) fn new(session: SessionId, revision: Revision, key: ArtifactKey) -> Self {
        Self {
            session,
            revision,
            key,
        }
    }
}
