use crate::{Compiler, CompilerResult};
use destack_artifact::ArtifactPayload;
use destack_repository::ProviderContext;

use destack_source::{ModuleId, TargetId};
use destack_repository::ProfileId;

impl Compiler {
    /// Build one module output.
    pub(crate) fn provide_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let output = self.generate_target_module_output(module, profile, &target, context)?;

        Ok(ArtifactPayload::ModuleOutput(output))
    }
}
