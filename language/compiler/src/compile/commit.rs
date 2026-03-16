use crate::{BuildDependency, BuildKey, Compiler};

impl Compiler {
    /// Commit the completed build dependency from the current workspace state.
    pub(crate) fn commit_completed_build(&self, build_key: &BuildKey) {
        let dependency = self.build_dependency_for_key(build_key);
        self.set_build_dependency(build_key, dependency);
    }

    /// Record one dependency stamp for one completed build key.
    fn set_build_dependency(&self, build_key: &BuildKey, dependency: BuildDependency) {
        match (build_key, dependency) {
            (BuildKey::Artifact(artifact_key), BuildDependency::Artifact(artifact_dependency)) => {
                self.program
                    .artifacts
                    .set_dependency(artifact_key.clone(), artifact_dependency);
            }
            (BuildKey::Output(output_key), BuildDependency::Output(output_dependency)) => {
                self.program
                    .outputs
                    .set_dependency(output_key.clone(), output_dependency);
            }
            _ => {}
        }
    }
}
