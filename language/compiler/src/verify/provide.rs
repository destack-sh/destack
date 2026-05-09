use destack_artifact::{ArtifactKey, ArtifactPayload, MirVerified};
use destack_source::{ModuleId, TargetId};
use destack_workspace::{ProfileId, ProviderContext};

use crate::verify::VerifyState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build verified MIR marker for one module and target.
    pub(crate) fn provide_mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = VerifyState::new(module, profile, target, context);

        state
            .context
            .require(ArtifactKey::mir_lowered(
                state.module,
                state.profile,
                state.target,
            ))
            .map_err(CompilerError::from)?;

        Ok(ArtifactPayload::MirVerified(MirVerified))
    }
}
