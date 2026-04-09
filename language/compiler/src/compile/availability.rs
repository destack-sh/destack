use destack_artifact::{ArtifactKey, ArtifactStatus};
use destack_workspace::Revision;

use crate::{
    ArtifactCompileKeyExt, ArtifactRequirement, Compiler, CompilerContext, RequirementError,
    RequirementSet,
};

impl Compiler {
    /// Require one artifact key to be available, returning an error if it is not ready or failed.
    pub(crate) fn require_artifact(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<(), RequirementError> {
        let requirement = ArtifactRequirement {
            anchor: artifact_key.anchor(),
            version: self.artifact_version_for_revision(revision, &artifact_key),
            error: None,
        };

        if self.current_requirement_is_recorded(&requirement) {
            return Ok(());
        }

        // exact status
        match self.artifacts.status(&requirement.version) {
            ArtifactStatus::Ready => {
                self.record_current_requirement(requirement);
                Ok(())
            }
            ArtifactStatus::Failed => Err(RequirementError::Failed {
                requirement: RequirementSet::one(requirement),
            }),
            ArtifactStatus::Missing => Err(RequirementError::NotReady {
                requirement: RequirementSet::one(requirement),
            }),
        }
    }

    /// Return whether one artifact key is currently available.
    #[cfg(test)]
    pub(crate) fn artifact_key_is_available(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> bool {
        let version = self.artifact_version_for_revision(revision, artifact_key);

        self.artifacts.status(&version) == ArtifactStatus::Ready
    }
}

impl CompilerContext<'_> {
    /// Require one artifact key to be available in this pinned revision.
    pub(crate) fn require_artifact(
        &self,
        artifact_key: ArtifactKey,
    ) -> Result<(), RequirementError> {
        self.compiler()
            .require_artifact(self.revision(), artifact_key)
    }
}
