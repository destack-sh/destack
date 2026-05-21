use destack_artifact::ArtifactPayload;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ProviderContext;

use crate::resolve::state::ResolveState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build resolved import targets for one module.
    pub(crate) fn provide_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, profile)
            .map_err(CompilerError::from)?;
        let imported = artifacts
            .dir_imported(module, profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, profile)
            .map_err(CompilerError::from)?;
        let environment = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;

        // build expanded resolve inputs
        let patches = std::slice::from_ref(&expanded.patch);
        let view = dir::View::with_patches(&parsed.tree, patches);
        let bindings = expanded.binding_table(&bound);
        let dependencies = expanded.dependency_table(&imported);
        let mut state = ResolveState::new(
            artifacts,
            profile,
            module,
            view,
            bindings,
            dependencies,
            self.strings(),
        );

        // collect import and re-export clauses from active roots
        state.walk(&expanded.roots);

        // resolve dependency clauses through export tables
        state.resolve_dependency_clauses()?;

        // resolve profile globals through global tables
        state.resolve_profile_globals(&environment.globals)?;

        // emit recoverable resolve diagnostics
        for diagnostic in state.take_diagnostics() {
            self.emit_diagnostic(context, diagnostic)?;
        }

        // publish resolved DIR
        let resolved = state.finish();

        Ok(ArtifactPayload::DirResolved(resolved))
    }
}
