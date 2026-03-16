use crate::timing::tags;
use crate::{BuildKey, BuildRequirementError, Compiler, ExecuteError, ExecuteResult};

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
            .set_dir_patched(module, profile, payload);

        Ok(())
    }

    /// Ensure patched DIR exists for a module.
    pub fn require_dir_patched(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirPatched {
            module,
            profile,
        }))
    }
}
