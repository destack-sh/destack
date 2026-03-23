use destack_artifact::{ArtifactDependency, ArtifactKey};

use crate::Compiler;

impl Compiler {
    /// Commit one completed artifact dependency.
    pub(crate) fn commit_completed_artifact(
        &self,
        artifact_key: &ArtifactKey,
        dependency: ArtifactDependency,
    ) {
        self.set_artifact_dependency(artifact_key, dependency);
    }

    /// Record one dependency stamp for one completed artifact key.
    fn set_artifact_dependency(&self, artifact_key: &ArtifactKey, dependency: ArtifactDependency) {
        self.artifacts
            .set_dependency(artifact_key.clone(), dependency);
    }
}
