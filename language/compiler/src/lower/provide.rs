use destack_artifact::ArtifactPayload;
use destack_source::{ModuleId, TargetId};
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerResult, LowerError};

impl Compiler {
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
