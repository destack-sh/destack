use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, DirMaterialized, MirLowered, MirVerified,
};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::elaborate::ElaborateState;
use crate::verify::VerifyState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect the artifacts elaborating one module's MIR reads.
    pub(crate) fn collect_mir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_materialized(module, profile));
        dependencies.require(ArtifactKey::mir_lowered(module, profile, target));
        dependencies.require(ArtifactKey::mir_verified(module, profile, target));

        Ok(dependencies)
    }

    /// Instantiate the demanded bodies and elaborate destruction and safepoints over them.
    pub(crate) fn provide_mir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let materialized = artifacts
            .read::<DirMaterialized>((module, profile))
            .map_err(CompilerError::from)?;
        let lowered = artifacts
            .read::<MirLowered>((module, profile, target))
            .map_err(CompilerError::from)?;
        let verified = artifacts
            .read::<MirVerified>((module, profile, target))
            .map_err(CompilerError::from)?;

        // give every demanded specialization its body
        let instantiated = self.instantiate_module(
            module,
            &materialized,
            lowered.clone(),
            &artifacts,
            profile,
            target,
        )?;

        // analyze the specializations their templates were verified for
        let mut analyses = VerifyState::over(
            &instantiated.tree,
            &instantiated.drops,
            &instantiated.accesses,
            &lowered.dispatch,
            &instantiated.effects,
            instantiated.layout,
        );
        analyses.verify_functions(&instantiated.specializations);
        if let Some(error) = analyses.take_errors().pop() {
            return Err(CompilerError::Internal {
                message: format!("a verified template failed at an instance: {error:?}"),
            });
        }

        // merge the templates' retention and safepoints with the instances'
        let mut retention = verified.retention.clone();
        retention.extend(analyses.take_retention());
        retention.sort();
        let mut safepoints = verified.safepoints.clone();
        safepoints.extend(analyses.take_safepoints());
        safepoints.sort();

        // plan destruction and safepoints over the closed bodies
        let mut state = ElaborateState::new(instantiated, self.strings());
        state.elaborate(&retention, &safepoints)?;

        Ok(ArtifactPayload::MirElaborated(Arc::new(state.finish())))
    }
}
