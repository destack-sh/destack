use crate::resolve::ResolveState;
use crate::{Compiler, CompilerResult};
use destack_artifact::ArtifactPayload;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

impl Compiler {
    /// Build the language environment for one profile.
    pub(crate) fn provide_language_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ResolveState::profile(profile, context);

        panic!(
            "language environment provider is not wired yet for {:?} profile {:?}",
            state.context.artifact_key(),
            state.profile,
        )
    }

    /// Build the library environment for one profile.
    pub(crate) fn provide_ambient_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ResolveState::profile(profile, context);

        panic!(
            "ambient environment provider is not wired yet for {:?} profile {:?}",
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

        panic!(
            "DIR export provider is not wired yet for {:?} module {:?} profile {:?}",
            state.context.artifact_key(),
            state.module,
            state.profile,
        )
    }
}
