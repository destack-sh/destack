use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirVerified};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};
use std::sync::Arc;

use crate::verify::VerifyState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for the verified MIR marker of one module and target.
    pub(crate) fn collect_mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::mir_lowered(module, profile, target));

        Ok(dependencies)
    }

    /// Build verified MIR marker for one module and target.
    pub(crate) fn provide_mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let lowered = artifacts
            .mir_lowered(module, profile, target)
            .map_err(CompilerError::from)?;
        let mut state = VerifyState::new(module, profile, target, context, &lowered);

        state.check_ownership();

        // emit every diagnostic and fail the verified artifact
        let mut diagnostics = state.take_diagnostics();
        let Some(diagnostic) = diagnostics.pop() else {
            return Ok(ArtifactPayload::MirVerified(Arc::new(MirVerified)));
        };
        for diagnostic in diagnostics {
            state.context.emit(diagnostic.as_ref())?;
        }

        Err(CompilerError::Diagnostic(diagnostic))
    }
}
