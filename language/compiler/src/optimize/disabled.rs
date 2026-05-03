use destack_artifact::{ArtifactKey, ArtifactPayload, MirOptimized};
use destack_source::{ModuleId, TargetId};
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build optimized MIR for one module and target.
    pub(crate) fn provide_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifact_key = ArtifactKey::mir_optimized(module, profile, target);
        let lowered = self
            .mir_lowered(context, module, profile, &target)
            .map_err(CompilerError::from)?;
        let payload = MirOptimized {
            tree: lowered.tree.clone(),
            strings: lowered.strings.clone(),
        };

        assert_eq!(
            artifact_key,
            context.artifact_key(),
            "compiler attempted to provide the wrong artifact"
        );

        Ok(ArtifactPayload::MirOptimized(payload))
    }
}
