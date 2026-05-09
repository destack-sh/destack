use destack_artifact::{ArtifactKey, ArtifactPayload, DirChecked};
use destack_dir::{CaptureTable, LayoutTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::check::CheckState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build checked DIR side tables for one declared module.
    pub(crate) fn provide_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = CheckState::new(module, profile, context);
        state
            .context
            .require(ArtifactKey::dir_exported(state.module, state.profile))
            .map_err(CompilerError::from)?;
        let expanded = self
            .dir_expanded(state.context, state.module, state.profile)
            .map_err(CompilerError::from)?;
        let payload = DirChecked {
            types: TypeTable::from_base(&expanded.types),
            layouts: LayoutTable::new(state.module),
            captures: CaptureTable::new(),
        };

        Ok(ArtifactPayload::DirChecked(payload))
    }
}
