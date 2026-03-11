use crate::{
    BuildDependency, BuildKey, BuildRequirement, BuildRequirementError, BuildRequirementSet,
    Compiler, Task, TaskStatus,
};
use destack_workspace::{ArtifactKey, OutputKey};

impl Compiler {
    /// Require one build key to be available, returning an error if it's not ready or has failed.
    pub(crate) fn require_build_key(
        &self,
        build_key: BuildKey,
    ) -> Result<(), BuildRequirementError> {
        let anchor = Task::new(build_key.clone()).anchor();

        if self.build_key_is_available(&build_key) {
            return Ok(());
        }

        if let Some(TaskStatus::Failed { error }) = self.queue.find_task_status(&build_key) {
            return Err(BuildRequirementError::Failed {
                requirement: BuildRequirementSet::one(BuildRequirement {
                    anchor: anchor.clone(),
                    key: build_key.clone(),
                    dependency: self.build_dependency_for_key(&build_key),
                    error: Some(Box::new(error)),
                }),
            });
        }

        Err(BuildRequirementError::NotReady {
            requirement: BuildRequirementSet::one(BuildRequirement {
                anchor,
                key: build_key.clone(),
                dependency: self.build_dependency_for_key(&build_key),
                error: None,
            }),
        })
    }

    /// Return whether one build key has a published product available.
    pub(crate) fn build_key_is_available(&self, build_key: &BuildKey) -> bool {
        let dependency = self.build_dependency_for_key(build_key);

        self.build_key_satisfies_dependency(build_key, dependency)
    }

    /// Return whether one build key satisfies one specific dependency.
    pub(crate) fn build_key_satisfies_dependency(
        &self,
        build_key: &BuildKey,
        dependency: BuildDependency,
    ) -> bool {
        match build_key {
            BuildKey::Artifact(artifact_key) => {
                self.artifact_is_available(artifact_key, dependency)
            }
            BuildKey::Output(output_key) => self.output_is_available(output_key, dependency),
        }
    }

    /// Return whether one semantic artifact is published.
    pub(crate) fn artifact_is_available(
        &self,
        artifact_key: &ArtifactKey,
        dependency: BuildDependency,
    ) -> bool {
        let BuildDependency::Artifact(expected_dependency) = dependency else {
            return false;
        };

        if self.program.artifacts.dependency(artifact_key) != Some(expected_dependency) {
            return false;
        }

        match artifact_key {
            ArtifactKey::LanguageEnvironment { profile } => self
                .program
                .artifacts
                .language_environment(*profile)
                .is_some(),
            ArtifactKey::IntrinsicEnvironment { profile } => self
                .program
                .artifacts
                .intrinsic_environment(*profile)
                .is_some(),
            ArtifactKey::LibEnvironment { profile } => {
                self.program.artifacts.lib_environment(*profile).is_some()
            }
            ArtifactKey::Ast { module } => self.program.artifacts.ast(*module).is_some(),
            ArtifactKey::DirBase { module } => self.program.artifacts.dir_base(*module).is_some(),
            ArtifactKey::DirPrepared { module, profile } => self
                .program
                .artifacts
                .dir_prepared(*module, *profile)
                .is_some(),
            ArtifactKey::DirResolved { module, profile } => self
                .program
                .artifacts
                .dir_resolved(*module, *profile)
                .is_some(),
            ArtifactKey::DirDeclared { module, profile } => self
                .program
                .artifacts
                .dir_declared(*module, *profile)
                .is_some(),
            ArtifactKey::DirInterface { module, profile } => self
                .program
                .artifacts
                .dir_interface(*module, *profile)
                .is_some(),
            ArtifactKey::DirAnalyzed { module, profile } => self
                .program
                .artifacts
                .dir_analyzed(*module, *profile)
                .is_some(),
            ArtifactKey::DirElaborated { module, profile } => self
                .program
                .artifacts
                .dir_elaborated(*module, *profile)
                .is_some(),
            ArtifactKey::DirPatched { module, profile } => self
                .program
                .artifacts
                .dir_patched(*module, *profile)
                .is_some(),
            ArtifactKey::Mir {
                module,
                profile,
                target,
            } => self
                .program
                .artifacts
                .mir(*module, *profile, target)
                .is_some(),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self
                .program
                .artifacts
                .optimized_mir(*module, *profile, target)
                .is_some(),
        }
    }

    /// Return whether one output product is published.
    pub(crate) fn output_is_available(
        &self,
        output_key: &OutputKey,
        dependency: BuildDependency,
    ) -> bool {
        let BuildDependency::Output(expected_dependency) = dependency else {
            return false;
        };

        self.program.outputs.dependency(output_key) == Some(expected_dependency)
            && self.program.outputs.contains_key(output_key)
    }
}
