use destack_artifact::{ArtifactKey, ArtifactPayload, DirExported};
use destack_dir::ExportTable;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::export::ExportState;
use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Build exported DIR for one module.
    pub(crate) fn provide_dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ExportState::new(module, profile, context);
        state
            .context
            .require(ArtifactKey::dir_expanded(state.module, state.profile))?;

        Ok(ArtifactPayload::DirExported(DirExported {
            exports: ExportTable::new(),
        }))
    }
}
