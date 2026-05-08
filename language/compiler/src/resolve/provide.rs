use crate::resolve::ResolveState;
use crate::{Compiler, CompilerResult};
use destack_artifact::{ArtifactKey, ArtifactPayload, DirExpanded};
use destack_core::StringPool;
use destack_dir::{Patch, SymbolTable};
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

    /// Build imported DIR for one module.
    pub(crate) fn provide_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ResolveState::module(profile, context);
        self.require_dir_declared(state.context, module, state.profile)?;

        todo!(
            "DIR import provider is unavailable for {:?} module {:?} profile {:?}",
            state.context.artifact_key(),
            module,
            state.profile,
        )
    }

    /// Build expanded DIR for one module.
    pub(crate) fn provide_dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ResolveState::module(profile, context);
        self.require_dir_imported(state.context, module, state.profile)?;
        let declared = self.require_dir_declared(state.context, module, state.profile)?;

        let artifact_key = ArtifactKey::dir_expanded(module, state.profile);
        assert_eq!(
            artifact_key,
            state.context.artifact_key(),
            "compiler attempted to provide the wrong artifact"
        );

        Ok(ArtifactPayload::DirExpanded(DirExpanded {
            patch: Patch::new(&declared.tree),
            strings: StringPool::new(),
            symbols: SymbolTable::new(module),
        }))
    }

    /// Build exported DIR for one module.
    pub(crate) fn provide_dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = ResolveState::module(profile, context);
        self.require_dir_expanded(state.context, module, state.profile)?;

        todo!(
            "DIR export provider is unavailable for {:?} module {:?} profile {:?}",
            state.context.artifact_key(),
            module,
            state.profile,
        )
    }
}
