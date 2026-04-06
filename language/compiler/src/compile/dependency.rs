use destack_artifact::{ArtifactKey, ArtifactStamp, ArtifactVersion};
use destack_workspace::Revision;

use crate::{Compiler, CompilerContext};

impl Compiler {
    /// Return the exact artifact version for one artifact key in one explicit revision.
    #[cfg(test)]
    pub(crate) fn artifact_version_for_revision(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> ArtifactVersion {
        let stamp = self.artifact_stamp_for_revision(revision, artifact_key);

        ArtifactVersion::new(*artifact_key, stamp)
    }

    /// Return the current artifact stamp for one artifact key in one explicit revision.
    pub(crate) fn artifact_stamp_for_revision(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> ArtifactStamp {
        self.repository.artifact_stamp(revision, artifact_key)
    }

    /// Return the current artifact stamp for one artifact key in the active execution scope.
    pub(crate) fn artifact_stamp_for_key(&self, artifact_key: &ArtifactKey) -> ArtifactStamp {
        self.current_context().artifact_stamp(artifact_key)
    }

    /// Return the exact artifact version for one artifact key in the active execution scope.
    pub(crate) fn artifact_version_for_key(&self, artifact_key: &ArtifactKey) -> ArtifactVersion {
        let stamp = self.artifact_stamp_for_key(artifact_key);

        ArtifactVersion::new(*artifact_key, stamp)
    }
}

impl CompilerContext<'_> {
    /// Return the current artifact stamp for one artifact key in this pinned revision.
    pub(crate) fn artifact_stamp(&self, artifact_key: &ArtifactKey) -> ArtifactStamp {
        self.compiler()
            .repository
            .artifact_stamp(self.revision(), artifact_key)
    }

    /// Return the exact artifact version for one artifact key in this pinned revision.
    pub(crate) fn artifact_version(&self, artifact_key: &ArtifactKey) -> ArtifactVersion {
        ArtifactVersion::new(*artifact_key, self.artifact_stamp(artifact_key))
    }
}
