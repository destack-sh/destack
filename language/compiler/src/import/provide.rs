use crate::{Compiler, CompilerResult};
use destack_artifact::{ArtifactKey, ArtifactPayload, DirImported};
use destack_dir::DependencyTable;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::import::ImportState;

impl Compiler {
    /// Build the global environment for one profile.
    pub(crate) fn provide_global_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        todo!(
            "global environment provider is unavailable for {:?} profile {:?}",
            context.artifact_key(),
            profile,
        )
    }

    /// Build imported DIR for one module.
    pub(crate) fn provide_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ImportState::new(module, profile, context);
        state
            .context
            .require(ArtifactKey::dir_declared(state.module, state.profile))?;

        Ok(ArtifactPayload::DirImported(DirImported {
            dependencies: DependencyTable::new(state.module),
        }))
    }
}
