use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirLowered, MirVerified,
};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::elaborate::ElaborateState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for elaborated MIR of one module and target.
    pub(crate) fn collect_mir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::mir_lowered(module, profile, target));
        dependencies.require(ArtifactKey::mir_verified(module, profile, target));

        Ok(dependencies)
    }

    /// Build elaborated MIR for one module and target.
    pub(crate) fn provide_mir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let lowered = artifacts
            .read::<MirLowered>((module, profile, target))
            .map_err(CompilerError::from)?;
        artifacts
            .read::<MirVerified>((module, profile, target))
            .map_err(CompilerError::from)?;
        let mut state = ElaborateState::new((*lowered).clone(), self.strings());

        state.generate_destructors();
        state.insert_drops();

        Ok(ArtifactPayload::MirElaborated(Arc::new(state.finish())))
    }
}
