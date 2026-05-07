use crate::resolve::ResolveState;
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
        let state = ResolveState::profile(profile, context);

        todo!(
            "global environment provider is unavailable for {:?} profile {:?}",
            state.context.artifact_key(),
            state.profile,
        )
    }

    /// Build exported DIR for one module.
    pub(crate) fn provide_dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ResolveState::module(module, profile, context);

        todo!(
            "DIR export provider is unavailable for {:?} module {:?} profile {:?}",
            state.context.artifact_key(),
            state.module,
            state.profile,
        )
    }
}
