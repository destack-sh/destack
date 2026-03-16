use std::sync::Arc;

use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ModuleDir, ProfileId};

use crate::{
    BuildKey, BuildRequirement, BuildRequirementError, BuildRequirementSet, Compiler,
    DiagnosticAnchor,
};

impl Compiler {
    /// Read one committed base DIR artifact when available.
    pub(crate) fn artifact_dir_base(&self, module_id: ModuleId) -> Option<Arc<ModuleDir>> {
        self.program.artifacts.dir_base(module_id)
    }

    /// Build one failed requirement set for one missing committed artifact.
    fn missing_artifact_requirement(&self, key: ArtifactKey) -> BuildRequirementSet {
        let anchor = match &key {
            ArtifactKey::DirBase { module }
            | ArtifactKey::DirDeclared { module, .. }
            | ArtifactKey::DirInterface { module, .. }
            | ArtifactKey::DirAnalyzed { module, .. } => DiagnosticAnchor::from(*module),
            _ => DiagnosticAnchor::Global,
        };
        let dependency = self.build_dependency_for_key(&BuildKey::Artifact(key.clone()));
        let requirement = BuildRequirement::new(anchor, BuildKey::Artifact(key), dependency);

        BuildRequirementSet::one(requirement)
    }

    /// Read one committed DIR artifact for one exact key.
    pub(crate) fn require_artifact_dir(
        &self,
        key: ArtifactKey,
    ) -> Result<Arc<ModuleDir>, BuildRequirementError> {
        let Some(module_id) = key.module_id() else {
            return Err(BuildRequirementError::Failed {
                requirement: self.missing_artifact_requirement(key),
            });
        };

        let Some(profile) = key.profile_id() else {
            return Err(BuildRequirementError::Failed {
                requirement: self.missing_artifact_requirement(key),
            });
        };

        match &key {
            ArtifactKey::DirResolved { .. } => self.require_dir_resolved(module_id, profile)?,
            ArtifactKey::DirDeclared { .. } => self.require_dir_declared(module_id, profile)?,
            ArtifactKey::DirInterface { .. } => self.require_dir_interface(module_id, profile)?,
            ArtifactKey::DirAnalyzed { .. } => self.require_dir_analyzed(module_id, profile)?,
            ArtifactKey::DirElaborated { .. } => self.require_dir_elaborated(module_id, profile)?,
            ArtifactKey::DirPatched { .. } => self.require_dir_patched(module_id, profile)?,
            ArtifactKey::DirPrepared { .. } => self.require_dir_prepared(module_id, profile)?,
            ArtifactKey::DirBase { .. } => {}
            _ => {
                return Err(BuildRequirementError::Failed {
                    requirement: self.missing_artifact_requirement(key),
                });
            }
        }

        let Some(dir) = self.program.artifacts.dir(&key) else {
            return Err(BuildRequirementError::Failed {
                requirement: self.missing_artifact_requirement(key),
            });
        };

        Ok(dir)
    }

    /// Read one committed elaborated DIR artifact.
    pub(crate) fn require_artifact_dir_elaborated(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<ModuleDir>, BuildRequirementError> {
        self.require_dir_elaborated(module_id, profile)?;

        let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) else {
            return Err(BuildRequirementError::Failed {
                requirement: self.missing_artifact_requirement(ArtifactKey::DirElaborated {
                    module: module_id,
                    profile,
                }),
            });
        };

        Ok(dir)
    }

    /// Read one committed base DIR artifact.
    pub(crate) fn require_artifact_dir_base(
        &self,
        module_id: ModuleId,
    ) -> Result<Arc<ModuleDir>, BuildRequirementError> {
        let Some(dir) = self.artifact_dir_base(module_id) else {
            return Err(BuildRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::DirBase { module: module_id }),
            });
        };

        Ok(dir)
    }

    /// Require one remote DIR artifact when a read targets a different module id.
    pub(crate) fn require_remote_artifact_dir(
        &self,
        local_module_id: ModuleId,
        target_module_id: ModuleId,
        profile: ProfileId,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> Result<(), BuildRequirementError> {
        // local reads do not need committed artifact gating
        if local_module_id == target_module_id {
            return Ok(());
        }

        let key = artifact_key(target_module_id, profile);
        self.require_artifact_dir(key).map(|_| ())
    }
}
