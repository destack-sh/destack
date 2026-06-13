use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::{Compiler, CompilerResult, LowerError};

impl Compiler {
    /// Collect inputs for MIR of one module and target.
    pub(crate) fn collect_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        _target: TargetId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_elaborated(module, profile));

        Ok(dependencies)
    }

    /// Provide MIR for one module and target.
    pub(crate) fn provide_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        Err(LowerError::Internal {
            anchor: module.into(),
            module,
            message: format!("MIR lower is disabled for profile {profile:?}, target {target:?}"),
        }
        .into())
    }
}
