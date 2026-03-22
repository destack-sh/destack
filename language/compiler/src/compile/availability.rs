use destack_workspace::{ArtifactDependency, ArtifactKey};

use crate::{
    ArtifactRequirement, ArtifactRequirementError, ArtifactRequirementSet, ArtifactTaskKeyExt,
    Compiler, TaskId, TaskStatus,
};

impl Compiler {
    /// Require one artifact key to be available, returning an error if it is not ready or failed.
    pub(crate) fn require_artifact(
        &self,
        artifact_key: ArtifactKey,
    ) -> Result<(), ArtifactRequirementError> {
        let anchor = artifact_key.anchor();
        let requirement = ArtifactRequirement {
            anchor: anchor.clone(),
            key: artifact_key.clone(),
            dependency: self.artifact_dependency_for_key(&artifact_key),
            error: None,
        };

        if self.current_requirement_is_recorded(&requirement) {
            return Ok(());
        }

        if self.artifact_satisfies_dependency(&artifact_key, requirement.dependency)
            && self.artifact_requirements_are_satisfied(&artifact_key)
        {
            self.record_current_requirement(requirement);
            return Ok(());
        }

        if let Some(TaskStatus::Failed { error }) = self.queue.find_task_status(&artifact_key) {
            return Err(ArtifactRequirementError::Failed {
                requirement: ArtifactRequirementSet::one(ArtifactRequirement {
                    error: Some(Box::new(error)),
                    ..requirement.clone()
                }),
            });
        }

        Err(ArtifactRequirementError::NotReady {
            requirement: ArtifactRequirementSet::one(requirement),
        })
    }

    /// Return whether one artifact key is currently available.
    pub(crate) fn artifact_key_is_available(&self, artifact_key: &ArtifactKey) -> bool {
        let dependency = self.artifact_dependency_for_key(artifact_key);

        self.artifact_satisfies_dependency(artifact_key, dependency)
            && self.artifact_requirements_are_satisfied(artifact_key)
    }

    /// Return whether one artifact key still satisfies its last completed exact requirements.
    pub(crate) fn artifact_requirements_are_satisfied(&self, artifact_key: &ArtifactKey) -> bool {
        let Some(task_id) = self.queue.find_task_id(artifact_key) else {
            return true;
        };

        let task_count = self.queue.task_count();
        let mut visiting = vec![false; task_count];
        let mut satisfied = vec![None; task_count];

        self.task_requirements_are_satisfied(task_id, &mut visiting, &mut satisfied)
    }

    /// Return whether one exact requirement set is currently satisfied.
    pub(crate) fn requirements_are_satisfied(&self, requirements: &[ArtifactRequirement]) -> bool {
        let task_count = self.queue.task_count();
        let mut visiting = vec![false; task_count];
        let mut satisfied = vec![None; task_count];

        self.requirement_list_is_satisfied(requirements, &mut visiting, &mut satisfied)
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

        let is_satisfied =
            self.requirement_list_is_satisfied(&handle.final_requirements, visiting, satisfied);

        visiting[task_index] = false;
        satisfied[task_index] = Some(is_satisfied);

        is_satisfied
    }

    /// Return whether one requirement list is currently satisfied.
    fn requirement_list_is_satisfied(
        &self,
        requirements: &[ArtifactRequirement],
        visiting: &mut [bool],
        satisfied: &mut [Option<bool>],
    ) -> bool {
        requirements.iter().all(|requirement| {
            self.artifact_satisfies_dependency(&requirement.key, requirement.dependency)
                && self
                    .queue
                    .find_task_id(&requirement.key)
                    .map(|required_task_id| {
                        self.task_requirements_are_satisfied(required_task_id, visiting, satisfied)
                    })
                    .unwrap_or(true)
        })
    }

    /// Return whether one artifact key satisfies one specific dependency.
    pub(crate) fn artifact_satisfies_dependency(
        &self,
        artifact_key: &ArtifactKey,
        dependency: ArtifactDependency,
    ) -> bool {
        self.artifact_is_published(artifact_key, dependency)
    }

    /// Return whether one artifact is published with the expected dependency stamp.
    pub(crate) fn artifact_is_published(
        &self,
        artifact_key: &ArtifactKey,
        dependency: ArtifactDependency,
    ) -> bool {
        if self.program.artifacts.dependency(artifact_key) != Some(dependency) {
            return false;
        }

        match artifact_key {
            ArtifactKey::ModuleGraph { profile } => {
                self.program.artifacts.module_graph(*profile).is_some()
            }
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
            ArtifactKey::LibraryEnvironment { profile } => self
                .program
                .artifacts
                .library_environment(*profile)
                .is_some(),
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
            ArtifactKey::ModuleOutput { module, target } => self
                .program
                .artifacts
                .module_output(*module, target)
                .is_some(),
            ArtifactKey::PackageOutput { package, target } => self
                .program
                .artifacts
                .package_output(*package, target)
                .is_some(),
        }
    }
}
