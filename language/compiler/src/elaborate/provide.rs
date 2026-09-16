use std::sync::Arc;

use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirInstantiated};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::elaborate::ElaborateState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect the artifacts elaborating one module's MIR reads.
    pub(crate) fn collect_mir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> CompilerResult<ArtifactDependencySet> {
        // require instantiated bodies and ownership retention
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require_payload(ArtifactKey::mir_instantiated(module, profile, target));

        Ok(dependencies)
    }

    /// Elaborate ownership and concrete representations over instance bodies.
    pub(crate) fn provide_mir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // read instantiated bodies and ownership retention
        let artifacts = self.artifact_reader(context);
        let instantiated = artifacts
            .read::<MirInstantiated>((module, profile, target))
            .map_err(CompilerError::from)?;

        // elaborate destruction and physical layouts
        let mut state = ElaborateState::new(module, &instantiated, self.strings());
        state.elaborate(&instantiated.retention)?;

        Ok(ArtifactPayload::MirElaborated(Arc::new(state.finish())))
    }
}
