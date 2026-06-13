use std::iter;
use std::sync::Arc;

use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactSidecar};
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderContext, ProviderError, Revision};
use destack_source::{FileContent, ModuleId, ProfileId};
use indexmap::IndexSet;

use crate::resolve::state::ResolveState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for resolved import targets of one module.
    pub(crate) fn collect_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require(ArtifactKey::global_environment(profile));

        // the re-export frontier is derived from expanded DIR once built
        let targets = match self.module_clause_targets(module, profile, context.revision()) {
            Ok(targets) => targets,
            Err(CompilerError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error),
        };
        let artifacts = self.artifact_reader(context.revision());
        self.collect_exported_modules(targets, profile, &artifacts, &mut dependencies)?;

        Ok(dependencies)
    }

    /// Build resolved import targets for one module.
    pub(crate) fn provide_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let artifacts = self.artifact_reader(context.revision());
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

        // resolve every collected reference over the declared export closure
        state.resolve(&environment)?;

        // emit resolve stats before diagnostics are drained
        let mut stats = state.stats;
        stats.record_exports(state.exports.stats());
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

        Ok(ArtifactPayload::DirResolved(Arc::new(resolved)))
    }

    /// Declare exported modules reachable through re-exports.
    pub(crate) fn collect_exported_modules(
        &self,
        modules: impl IntoIterator<Item = ModuleId>,
        profile: ProfileId,
        artifacts: &ArtifactReader<'_>,
        dependencies: &mut ArtifactDependencySet,
    ) -> CompilerResult<()> {
        let mut seen = IndexSet::new();
        let mut frontier = modules.into_iter().collect::<Vec<_>>();

        // declare each reachable export, widening through the built ones
        while let Some(module) = frontier.pop() {
            if !seen.insert(module) {
                continue;
            }

            dependencies.require(ArtifactKey::dir_exported(module, profile));

            // follow re-export edges through exports that are already built
            match artifacts.dir_exported(module, profile) {
                Ok(exported) => frontier.extend(exported.reexport_modules()),
                Err(ProviderError::Blocked { .. }) => dependencies.mark_partial(),
                Err(error) => return Err(CompilerError::from(error)),
            }
        }

        Ok(())
    }

    /// Return the modules one module's import and re-export clauses target.
    fn module_clause_targets(
        &self,
        module: ModuleId,
        profile: ProfileId,
        revision: Revision,
    ) -> CompilerResult<Vec<ModuleId>> {
        let artifacts = self.artifact_reader(revision);
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

        // build the resolve view without walking references
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

        // module clauses are syntactic, so no reference walk is needed
        state.collect_module_clauses(&expanded.roots);

        Ok(state.module_clause_targets().collect())
    }
}
