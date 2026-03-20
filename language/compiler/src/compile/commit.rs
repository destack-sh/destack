use destack_workspace::{ArtifactDependency, ArtifactKey};

use crate::Compiler;

impl Compiler {
    /// Commit the completed artifact dependency from the current workspace state.
    pub(crate) fn commit_completed_artifact(&self, artifact_key: &ArtifactKey) {
        let dependency = self.artifact_dependency_for_key(artifact_key);
        self.set_artifact_dependency(artifact_key, dependency);
    }

    /// Record one dependency stamp for one completed artifact key.
    fn set_artifact_dependency(&self, artifact_key: &ArtifactKey, dependency: ArtifactDependency) {
        self.program
            .artifacts
            .set_dependency(artifact_key.clone(), dependency);
    }
}
