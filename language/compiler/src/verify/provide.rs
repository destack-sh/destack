use destack_artifact::{ArtifactKey, ArtifactPayload, MirVerified};
use destack_source::{ModuleId, TargetId};
use destack_workspace::{ProfileId, ProviderContext};

use crate::verify::{DropInsert, OwnershipCheck, VerifyState};
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
        let mut state = VerifyState::new(module, profile, target, context);

        state
            .context
            .require(ArtifactKey::mir_lowered(
                state.module,
                state.profile,
                state.target,
            ))
            .map_err(CompilerError::from)?;
        let lowered = self
            .mir_lowered(state.context, state.module, state.profile, &state.target)
            .map_err(CompilerError::from)?;
        let mut tree = lowered.tree.clone();

        OwnershipCheck.run(&mut tree, &mut state);
        if !state.has_errors() {
            DropInsert.run(&mut tree, &mut state);
        }

        for diagnostic in state.take_diagnostics() {
            state.context.emit(diagnostic.as_ref())?;
        }

        Ok(ArtifactPayload::MirVerified(MirVerified::from_tree(tree)))
    }
}
