use crate::{Compiler, CompilerResult};
use destack_artifact::ArtifactPayload;
use destack_workspace::ProviderContext;

use destack_artifact::ArtifactKey;
use destack_source::{ModuleId, TargetId};
use destack_workspace::{ProfileId, ProviderError};

impl Compiler {
    /// Build one module output.
    pub(crate) fn provide_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifact_key = ArtifactKey::module_output(module, target);
        let output = self.generate_target_module_output(module, profile, &target, context)?;
        assert_eq!(
            artifact_key,
            context.artifact_key(),
            "compiler attempted to provide the wrong artifact"
        );

        Ok(ArtifactPayload::ModuleOutput(output))
    }
    /// Require one module output.
    pub fn require_module_output(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), ProviderError> {
        let _ = profile;
        context.require(ArtifactKey::module_output(module, *target))?;

        Ok(())
    }
}
