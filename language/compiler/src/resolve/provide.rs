use std::iter;

use destack_artifact::{ArtifactKey, ArtifactPayload, ArtifactSidecar};
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderContext};
use destack_source::{FileContent, ModuleId, ProfileId};
use indexmap::IndexSet;

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
        // require provider inputs
        let artifacts = self.artifact_reader(context);
        artifacts
            .require_all(&[
                ArtifactKey::dir_expanded(module, profile),
                ArtifactKey::global_environment(profile),
            ])
            .map_err(CompilerError::from)?;

        // load provider inputs
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

        // collect source references, module clauses, and syntax language items
        state.collect_module_clauses(&expanded.roots);
        state.walk(&expanded.roots);

        // require exported modules read by resolve lookups
        let mut exported_modules = IndexSet::new();
        exported_modules.extend(state.module_clause_targets());
        exported_modules.extend(environment.globals.iter().copied());
        self.require_exported_modules(exported_modules, profile, &state.artifacts)?;

        // resolve explicit module clauses through exports
        state.resolve_module_clauses()?;

        // resolve globals through exports
        state.resolve_profile_globals(&environment.globals)?;

        // resolve source-visible language globals
        state.resolve_language_globals(&environment.language)?;

        // resolve namespace path references
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

    /// Require exported modules reachable through re-exports.
    fn require_exported_modules(
        &self,
        modules: impl IntoIterator<Item = ModuleId>,
        profile: ProfileId,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<()> {
        let mut seen = IndexSet::new();
        let mut pending = modules
            .into_iter()
            .filter(|module| seen.insert(*module))
            .collect::<Vec<_>>();

        // require one reachable frontier at a time
        while !pending.is_empty() {
            let requirements = pending
                .iter()
                .map(|module| ArtifactKey::dir_exported(*module, profile))
                .collect::<Vec<_>>();
            artifacts
                .require_all(&requirements)
                .map_err(CompilerError::from)?;

            let frontier = pending;
            pending = Vec::new();

            // discover the next re-export frontier
            for module in frontier {
                let exported = artifacts
                    .dir_exported(module, profile)
                    .map_err(CompilerError::from)?;

                for target in exported.reexport_modules() {
                    if seen.insert(target) {
                        pending.push(target);
                    }
                }
            }
        }

        Ok(())
    }
}
