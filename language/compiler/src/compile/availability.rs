use destack_artifact::{ArtifactKey, ArtifactStamp, ArtifactVersion};
use destack_workspace::Revision;

use crate::{
    ArtifactRequirement, ArtifactTaskKeyExt, Compiler, CompilerContext, Requirement,
    RequirementError, RequirementSet, TaskId, TaskStatus,
};

impl Compiler {
    /// Require one artifact key to be available, returning an error if it is not ready or failed.
    pub(crate) fn require_artifact(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<(), RequirementError> {
        let anchor = artifact_key.anchor();
        let requirement = ArtifactRequirement {
            anchor: anchor.clone(),
            key: artifact_key,
            stamp: self.artifact_stamp_for_revision(revision, &artifact_key),
            error: None,
        };

        if self.current_requirement_is_recorded(&requirement) {
            return Ok(());
        }

        if self.artifact_satisfies_stamp_for_revision(revision, &artifact_key, requirement.stamp) {
            self.record_current_requirement(requirement);
            return Ok(());
        }

        if let Some(TaskStatus::Failed { error }) =
            self.queue.find_task_status(revision, &artifact_key)
        {
            return Err(RequirementError::Failed {
                requirement: RequirementSet::one(ArtifactRequirement {
                    error: Some(Box::new(error)),
                    ..requirement.clone()
                }),
            });
        }

        if let Some(requirement) = self.file_requirements_for_artifact(revision, &artifact_key) {
            return Err(RequirementError::NotReady { requirement });
        }

        Err(RequirementError::NotReady {
            requirement: RequirementSet::one(requirement),
        })
    }

    /// Return whether one artifact key is currently available.
    pub(crate) fn artifact_key_is_available(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> bool {
        let stamp = self.artifact_stamp_for_revision(revision, artifact_key);

        self.artifact_satisfies_stamp_for_revision(revision, artifact_key, stamp)
    }

    /// Collect the transitive file requirements for one task.
    fn collect_file_requirements(
        &self,
        task_id: TaskId,
        revision: Revision,
        visiting: &mut [bool],
        requirements: &mut Vec<Requirement>,
    ) {
        let task_index = task_id.0 as usize;
        if std::mem::replace(&mut visiting[task_index], true) {
            return;
        }

        let handle = self.queue.get_task(task_id);

        if let TaskStatus::Yielded { requirement } = &handle.status {
            requirement.for_each(|requirement| match requirement {
                Requirement::Artifact(requirement) => {
                    if let Some(required_task_id) =
                        self.queue.find_task_id(revision, &requirement.key)
                    {
                        self.collect_file_requirements(
                            required_task_id,
                            revision,
                            visiting,
                            requirements,
                        );
                    }
                }
                Requirement::File(requirement) => {
                    let requirement = Requirement::File(requirement.clone());
                    if !requirements.contains(&requirement) {
                        requirements.push(requirement);
                    }
                }
            });
        }

        visiting[task_index] = false;
    }

    /// Return whether one artifact key satisfies one specific stamp.
    pub(crate) fn artifact_satisfies_stamp_for_revision(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        stamp: ArtifactStamp,
    ) -> bool {
        self.artifact_satisfies_stamp_for_revision_with_active(
            revision,
            artifact_key,
            stamp,
            &mut Vec::new(),
        )
    }

    /// Return whether one artifact key satisfies one specific stamp in one explicit revision.
    fn artifact_satisfies_stamp_for_revision_with_active(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        stamp: ArtifactStamp,
        active_versions: &mut Vec<ArtifactVersion>,
    ) -> bool {
        let version = ArtifactVersion::new(*artifact_key, stamp);
        if active_versions.contains(&version) {
            return self.artifact_is_published(&version);
        }

        if !self.artifact_is_published(&version) {
            return false;
        }

        let Some(dependencies) = self.artifacts.dependencies(&version) else {
            return true;
        };

        active_versions.push(version);
        let is_satisfied = dependencies.into_iter().all(|dependency| {
            let current_stamp = self.artifact_stamp_for_revision(revision, &dependency.key);
            if current_stamp != dependency.stamp {
                return false;
            }

            self.artifact_satisfies_stamp_for_revision_with_active(
                revision,
                &dependency.key,
                dependency.stamp,
                active_versions,
            )
        });
        active_versions.pop();

        is_satisfied
    }

    /// Return whether one exact artifact version is published.
    pub(crate) fn artifact_is_published(&self, version: &ArtifactVersion) -> bool {
        self.artifacts.contains(version)
    }

    /// Return the transitive file requirements blocking one yielded artifact in one revision, if any.
    fn file_requirements_for_artifact(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Option<RequirementSet> {
        let task_id = self.queue.find_task_id(revision, artifact_key)?;
        let task_count = self.queue.task_count();
        let mut visiting = vec![false; task_count];
        let mut requirements = Vec::new();

        self.collect_file_requirements(task_id, revision, &mut visiting, &mut requirements);

        match requirements.len() {
            0 => None,
            1 => requirements.into_iter().next().map(RequirementSet::One),
            _ => Some(RequirementSet::All(requirements)),
        }
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
