use crate::{
    BuildDependency, BuildKey, BuildRequirement, BuildRequirementError, BuildRequirementSet,
    Compiler, Task, TaskId, TaskStatus,
};
use destack_workspace::{ArtifactKey, OutputKey};

impl Compiler {
    /// Require one build key to be available, returning an error if it's not ready or has failed.
    pub(crate) fn require_build_key(
        &self,
        build_key: BuildKey,
    ) -> Result<(), BuildRequirementError> {
        let anchor = Task::new(build_key.clone()).anchor();
        let requirement = BuildRequirement {
            anchor: anchor.clone(),
            key: build_key.clone(),
            dependency: self.build_dependency_for_key(&build_key),
            error: None,
        };

        if self.current_requirement_is_recorded(&requirement) {
            return Ok(());
        }

        if self.build_key_satisfies_dependency(&build_key, requirement.dependency)
            && self.build_key_requirements_are_satisfied(&build_key)
        {
            self.record_current_requirement(requirement);
            return Ok(());
        }

        if let Some(TaskStatus::Failed { error }) = self.queue.find_task_status(&build_key) {
            return Err(BuildRequirementError::Failed {
                requirement: BuildRequirementSet::one(BuildRequirement {
                    error: Some(Box::new(error)),
                    ..requirement.clone()
                }),
            });
        }

        Err(BuildRequirementError::NotReady {
            requirement: BuildRequirementSet::one(requirement),
        })
    }

    /// Return whether one build key has a published product available.
    pub(crate) fn build_key_is_available(&self, build_key: &BuildKey) -> bool {
        let dependency = self.build_dependency_for_key(build_key);

        self.build_key_satisfies_dependency(build_key, dependency)
            && self.build_key_requirements_are_satisfied(build_key)
    }

    /// Return whether one build key still satisfies its last completed exact requirements.
    pub(crate) fn build_key_requirements_are_satisfied(&self, build_key: &BuildKey) -> bool {
        let Some(task_id) = self.queue.find_task_id(build_key) else {
            return true;
        };

        let task_count = self.queue.task_count();
        let mut visiting = vec![false; task_count];
        let mut satisfied = vec![None; task_count];

        self.task_requirements_are_satisfied(task_id, &mut visiting, &mut satisfied)
    }

    /// Return whether one completed task still satisfies its last exact requirements.
    fn task_requirements_are_satisfied(
        &self,
        task_id: TaskId,
        visiting: &mut [bool],
        satisfied: &mut [Option<bool>],
    ) -> bool {
        let task_index = task_id.0 as usize;

        if let Some(is_satisfied) = satisfied[task_index] {
            return is_satisfied;
        }

        // converged requirement cycles should not invalidate availability on their own
        if std::mem::replace(&mut visiting[task_index], true) {
            return true;
        }

        let handle = self.queue.get_task(task_id);

        if !handle.status.is_final() {
            visiting[task_index] = false;
            return true;
        }

        let is_satisfied = handle.final_requirements.iter().all(|requirement| {
            self.build_key_satisfies_dependency(&requirement.key, requirement.dependency)
                && self
                    .queue
                    .find_task_id(&requirement.key)
                    .map(|required_task_id| {
                        self.task_requirements_are_satisfied(required_task_id, visiting, satisfied)
                    })
                    .unwrap_or(true)
        });

        visiting[task_index] = false;
        satisfied[task_index] = Some(is_satisfied);

        is_satisfied
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
            ArtifactKey::MirBase {
                module,
                profile,
                target,
            } => self
                .program
                .artifacts
                .mir_base(*module, *profile, target)
                .is_some(),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self
                .program
                .artifacts
                .mir_optimized(*module, *profile, target)
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
