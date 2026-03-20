use crate::timing::tags;
use crate::{ArtifactRequirementError, Compiler, ExecuteError, ExecuteResult};

use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ProfileId};

impl Compiler {
    /// Build patched DIR for one module.
    pub fn process_dir_patched(&self, module: ModuleId, profile: ProfileId) -> ExecuteResult<()> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<ExecuteError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;

        // reuse a persisted patched dir when it is still valid
        let artifact_key = ArtifactKey::dir_patched(module, profile);
        if self
            .load_published_artifact(artifact_key.clone(), |compiler| {
                compiler.load_dir_patched_image(module, module_version, profile)
            })
            .is_some()
        {
            return Ok(());
        }

        let _timing = self.timing_scope(tags::EXECUTE_MODULE_PATCH);
        self.require_intrinsic_environment(profile)
            .map_err(ExecuteError::from)?;
        let payload =
            self.execute_module_patch(module, profile, module_version, profile_version)?;
        if self.is_code_module(module) {
            self.stats.record_execute();
        }

        self.program
            .artifacts
            .publish(artifact_key.clone(), payload.clone());
        self.store_artifact(&artifact_key, &payload, |compiler, payload| {
            compiler.store_dir_patched_image(module, profile, payload)
        });

        Ok(())
    }

    /// Ensure patched DIR exists for a module.
    pub fn require_dir_patched(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), ArtifactRequirementError> {
        self.require_artifact(ArtifactKey::dir_patched(module, profile))
    }
}
