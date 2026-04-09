use destack_artifact::{ArtifactDependency, ArtifactKey, ArtifactStore, ArtifactVersion};

use crate::{ArtifactRequirement, Compiler, CompilerContext, RequirementSet};

impl Compiler {
    /// Publish one completed artifact payload through the compiler publication path.
    pub(crate) fn publish_artifact<T>(
        &self,
        artifact_key: ArtifactKey,
        payload: T,
        publish: impl FnOnce(&ArtifactStore, ArtifactVersion, T),
    ) {
        let version = self.artifact_version_for_key(&artifact_key);
        let dependencies = self.live_dependencies_for_current_artifact(&artifact_key);

        publish(&self.artifacts, version, payload);
        self.artifacts.publish_dependencies(&version, dependencies);
    }

    /// Return the live dependency proofs recorded for the current artifact build.
    pub(crate) fn live_dependencies_for_current_artifact(
        &self,
        artifact_key: &ArtifactKey,
    ) -> Vec<ArtifactDependency> {
        self.live_dependencies_for_artifact_requirements(artifact_key, &self.current_requirements())
    }

    /// Convert satisfied artifact requirements into live dependency proofs for one artifact.
    pub(crate) fn live_dependencies_for_artifact_requirements(
        &self,
        artifact_key: &ArtifactKey,
        requirements: &[ArtifactRequirement],
    ) -> Vec<ArtifactDependency> {
        let mut dependencies = Vec::new();

        for requirement in requirements {
            if let Some(dependency) =
                self.live_dependency_from_requirement(artifact_key, requirement.clone())
            {
                dependencies.push(dependency);
            }
        }

        dependencies
    }

    /// Convert one requirement set into live dependency proofs for one artifact.
    pub(crate) fn live_dependencies_for_requirement_set(
        &self,
        artifact_key: &ArtifactKey,
        requirements: &RequirementSet,
    ) -> Vec<ArtifactDependency> {
        let mut dependencies = Vec::new();

        requirements.for_each_artifact(|requirement| {
            if let Some(dependency) =
                self.live_dependency_from_requirement(artifact_key, requirement.clone())
            {
                dependencies.push(dependency);
            }
        });

        dependencies
    }

    /// Convert one satisfied task requirement into one live dependency proof.
    fn live_dependency_from_requirement(
        &self,
        artifact_key: &ArtifactKey,
        requirement: ArtifactRequirement,
    ) -> Option<ArtifactDependency> {
        if requirement.version.key == *artifact_key {
            return None;
        }

        Some(ArtifactDependency {
            version: requirement.version,
        })
    }
}

impl CompilerContext<'_> {
    /// Publish one completed artifact payload through the compiler publication path.
    pub(crate) fn publish_artifact<T>(
        &self,
        artifact_key: ArtifactKey,
        payload: T,
        publish: impl FnOnce(&ArtifactStore, ArtifactVersion, T),
    ) {
        let version = self.artifact_version(&artifact_key);
        let dependencies = self
            .compiler()
            .live_dependencies_for_current_artifact(&artifact_key);

        publish(&self.compiler().artifacts, version, payload);
        self.compiler()
            .artifacts
            .publish_dependencies(&version, dependencies);
    }
}
