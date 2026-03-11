use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ProfileId};

use crate::timing::tags;
use crate::{BuildKey, BuildRequirementError, Compiler, ElaborateError, ElaborateResult};

impl Compiler {
    /// Build elaborated DIR for one module.
    pub fn process_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> ElaborateResult<()> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<ElaborateError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::ELABORATE_MODULE_TRANSFORM);
        self.elaborate_module_transform(module, profile, module_version, profile_version)?;
        let _timing = self.timing_scope(tags::ELABORATE_MODULE_REIFY);
        self.elaborate_module_reify(module, profile, module_version, profile_version)?;
        if self.is_code_module(module) {
            self.stats.record_elaborate();
        }

        Ok(())
    }

    /// Ensure elaborated DIR exists for a module.
    pub fn require_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirElaborated {
            module,
            profile,
        }))
    }
}
