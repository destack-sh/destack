use tspp_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, MirElaborated};
use tspp_repository::{ProfileId, ProviderContext};
use tspp_source::{ModuleId, TargetId};

use crate::{Compiler, CompilerError, CompilerResult};

use super::optimize;

impl Compiler {
    /// Collect elaborated MIR for optimization.
    pub(crate) fn collect_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> CompilerResult<ArtifactDependencySet> {
        // require explicit ownership and concrete representations
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require_payload(ArtifactKey::mir_elaborated(module, profile, target));

        Ok(dependencies)
    }

    /// Optimize elaborated MIR for emission.
    pub(crate) fn provide_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // read the elaborated module
        let artifacts = self.artifact_reader(context);
        let elaborated = artifacts
            .read::<MirElaborated>((module, profile, target))
            .map_err(CompilerError::from)?;

        Ok(optimize(&elaborated)?.into())
    }
}
