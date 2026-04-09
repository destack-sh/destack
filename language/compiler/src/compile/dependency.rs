use destack_artifact::{ArtifactKey, ArtifactStamp, ArtifactVersion};
use destack_workspace::Revision;

use crate::{Compiler, CompilerContext};

impl Compiler {
    /// Return the exact artifact version for one artifact key in one explicit revision.
    pub(crate) fn artifact_version_for_revision(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> ArtifactVersion {
        self.with_active_artifact_versions(|cache| {
            if let Some((snapshot, active_revision, current_versions)) = cache {
                if active_revision == revision {
                    if let Some(version) = current_versions.get(artifact_key).copied() {
                        return version;
                    }

                    let version = snapshot.artifact_version(artifact_key);
                    current_versions.insert(*artifact_key, version);

                    return version;
                }
            }

            ArtifactVersion::new(
                *artifact_key,
                self.artifact_stamp_for_revision(revision, artifact_key),
            )
        })
    }

    /// Return the current artifact stamp for one artifact key in one explicit revision.
    pub(crate) fn artifact_stamp_for_revision(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> ArtifactStamp {
        self.repository.artifact_stamp(revision, artifact_key)
    }

    /// Return the exact artifact version for one artifact key in the active execution scope.
    pub(crate) fn artifact_version_for_key(&self, artifact_key: &ArtifactKey) -> ArtifactVersion {
        let revision = self.current_context().revision();

        self.artifact_version_for_revision(revision, artifact_key)
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
        self.compiler()
            .artifact_version_for_revision(self.revision(), artifact_key)
    }
}
