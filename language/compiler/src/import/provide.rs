use crate::{Compiler, CompilerResult};
use destack_artifact::ArtifactPayload;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

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
        let _ = self;

        todo!(
            "DIR import provider is unavailable for {:?} module {:?} profile {:?}",
            context.artifact_key(),
            module,
            profile,
        )
    }
}
