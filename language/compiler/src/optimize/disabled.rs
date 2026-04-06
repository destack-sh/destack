use destack_artifact::ArtifactKey;
use destack_source::{ModuleId, TargetId};
use destack_workspace::ProfileId;

use crate::{Compiler, CompilerContext, OptimizeError, OptimizeResult, RequirementError};

impl Compiler {
    /// Build optimized MIR for one module and target.
    pub fn process_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> OptimizeResult<()> {
        self.require_mir(module, profile, &target)?;

        Ok(())
    }

    /// Ensure optimized MIR exists for one module and target.
    pub fn require_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
        context: &CompilerContext<'_>,
    ) -> Result<(), RequirementError> {
        let _ = context;
        self.require_artifact(ArtifactKey::mir_optimized(module, profile, target.clone()))
    }
}
