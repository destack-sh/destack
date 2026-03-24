use crate::timing::tags;
use crate::{ArtifactRequirementError, Compiler, GenerateError, GenerateResult};

use destack_artifact::ArtifactKey;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, TargetId};

impl Compiler {
    /// Build one module artifact.
    pub fn process_module_artifact(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> GenerateResult<()> {
        let module_stamp = self.module_stamp(module);
        let profile_stamp = self.profile_stamp(profile);
        self.ensure_module_profile_matches::<GenerateError>(
            module_stamp.id,
            module_stamp.version,
            profile_stamp.id,
            profile_stamp.version,
        )?;
        let artifact_key = ArtifactKey::module_artifact(module, target.clone());

        let _timing = self.timing_scope(tags::GENERATE_MODULE);
        self.generate_target_module_artifact(
            module_stamp.id,
            profile_stamp.id,
            module_stamp.version,
            profile_stamp.version,
            &target,
        )?;
        let artifact = self
            .artifacts
            .module_artifact(module, &target)
            .ok_or_else(|| GenerateError::Internal {
                module,
                message: format!("missing module artifact for target '{target}'"),
            })?;
        self.store_artifact(&artifact_key, artifact.as_ref(), |_, _| Ok(()));
        self.stats.record_generate();

        Ok(())
    }
    /// Require one module artifact.
    pub fn require_module_artifact(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), ArtifactRequirementError> {
        let _ = profile;
        self.require_artifact(ArtifactKey::module_artifact(module, target.clone()))
    }
}
