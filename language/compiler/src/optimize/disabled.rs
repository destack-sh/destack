use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ProfileId, TargetId};

use crate::{BuildKey, BuildRequirementError, Compiler, OptimizeError, OptimizeResult};

impl Compiler {
    /// Build optimized MIR for one module and target.
    pub fn process_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> OptimizeResult<()> {
        // keep dependency and stale checks stable when optimize is disabled
        let module_stamp = self.module_stamp(module);
        let profile_stamp = self.profile_stamp(profile);
        self.ensure_module_profile_matches::<OptimizeError>(
            module_stamp.id,
            module_stamp.version,
            profile_stamp.id,
            profile_stamp.version,
        )?;

        self.require_mir(module_stamp.id, profile_stamp.id, &target)?;

        Ok(())
    }

    /// Ensure optimized MIR exists for one module and target.
    pub fn require_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::artifact(ArtifactKey::mir_optimized(
            module,
            profile,
            target.clone(),
        )))
    }
}
