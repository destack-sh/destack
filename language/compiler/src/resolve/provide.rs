use std::iter;

use destack_artifact::{ArtifactPayload, ArtifactSidecar};
use destack_dir as dir;
use destack_repository::ProviderContext;
use destack_source::{FileContent, ModuleId, ProfileId};

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
        let modules = expanded.module_table(&imported);
        let mut state = ResolveState::new(
            artifacts,
            profile,
            module,
            view,
            bindings,
            modules,
            self.strings(),
        );

        // resolve explicit module clauses through export tables
        state.resolve_module_clauses(&expanded.roots)?;

        // collect source references and syntax language items
        state.walk(&expanded.roots);

        // resolve profile globals through global tables
        state.resolve_profile_globals(&environment.globals)?;

        // resolve source-visible language globals
        state.resolve_language_globals(&environment.language)?;

        // resolve namespace path references through imported module surfaces
        state.resolve_path_references()?;

        // resolve syntax-required language item modules
        state.resolve_syntax_language_items(&environment.language)?;

        // emit resolve stats before diagnostics are drained
        let stats = state.stats;
        context.emit_sidecar(ArtifactSidecar::new(
            "metadata",
            iter::once(("phase", "resolve")),
            FileContent::Text {
                content: stats.render_metadata(),
            },
        ));

        // emit recoverable resolve diagnostics
        for diagnostic in state.take_diagnostics() {
            self.emit_diagnostic(context, diagnostic)?;
        }

        // publish resolved DIR
        let resolved = state.finish();

        Ok(ArtifactPayload::DirResolved(resolved))
    }
}
