use destack_artifact::{ArtifactKey, ArtifactPayload, DirChecked};
use destack_dir::{CaptureTable, LayoutTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::analyze::AnalyzeState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build checked DIR side tables for one declared module.
    pub(crate) fn provide_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = AnalyzeState::new(module, profile, context);
        let expanded = self
            .require_dir_expanded(state.context, state.module, state.profile)
            .map_err(CompilerError::from)?;
        self.require_dir_exported(state.context, state.module, state.profile)
            .map_err(CompilerError::from)?;
        let payload = DirChecked {
            types: TypeTable::from_base(&expanded.types),
            layouts: LayoutTable::new(state.module),
            captures: CaptureTable::new(),
        };

        let artifact_key = ArtifactKey::dir_checked(state.module, state.profile);
        assert_eq!(
            artifact_key,
            state.context.artifact_key(),
            "compiler attempted to provide the wrong artifact"
        );

        Ok(ArtifactPayload::DirChecked(payload))
    }
}
