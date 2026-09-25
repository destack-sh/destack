use std::sync::Arc;

use indexmap::IndexSet;
use tspp_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, DirBound, DirExpanded, DirExported,
    DirImported, DirParsed, DirView, EnvironmentBound,
};
use tspp_repository::{ArtifactReader, ProviderContext, ProviderError};
use tspp_source::{ModuleId, ProfileId};

use crate::resolve::state::ResolveState;
use crate::{Compiler, CompilerError, CompilerResult, ResolveError, ResolveWarning};

impl Compiler {
    /// Collect the dependencies of one resolved DIR build.
    pub(crate) fn collect_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_imported(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require(ArtifactKey::dir_exported(module, profile));
        dependencies.require(ArtifactKey::environment_bound(profile));

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
        let artifacts = self.artifact_reader(context);
        let key = (module, profile);
        let stages = DirView::expanded(
            artifacts.read::<DirParsed>(module)?,
            artifacts.read::<DirBound>(key)?,
            artifacts.read::<DirImported>(key)?,
            artifacts.read::<DirExpanded>(key)?,
        );
        let exported = artifacts
            .read::<DirExported>(key)
            .map_err(CompilerError::from)?;
        let environment = artifacts
            .read::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;

        // build expanded resolve inputs
        let parsed = stages.parsed();
        let view = stages.tree();

        let mut state = ResolveState::new(
            artifacts,
            profile,
            module,
            view,
            stages.bindings().clone(),
            stages.modules().clone(),
            self.strings(),
        );

        // collect source references, module clauses, and implied language items
        state.walk(&stages.expanded.roots);

        // resolve every collected reference over the declared export closure
        state.resolve(&environment, &exported)?;

        // pull in tree literals to the default builder
        if parsed.tree.has_tree_expressions() {
            state.record_tree_builder(environment.tree);
        }

        // record resolve stats before diagnostics are drained
        let mut stats = state.stats;
        stats.record_exports(state.exports.stats());
        stats.record(context);

        // emit resolve diagnostics
        for error in state.take_errors() {
            self.emit_diagnostic::<ResolveError>(context, error)?;
        }
        for warning in state.take_warnings() {
            self.emit_diagnostic::<ResolveWarning>(context, warning)?;
        }

        // publish resolved DIR
        let resolved = state.finish(&environment);

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

            dependencies.require_payload(ArtifactKey::dir_exported(module, profile));

            // follow re-export edges through exports that are already built
            match artifacts.read::<DirExported>((module, profile)) {
                Ok(exported) => frontier.extend(exported.reexport_modules()),
                Err(ProviderError::Blocked { .. }) => dependencies.mark_partial(),
                Err(error) => return Err(CompilerError::from(error)),
            }
        }

        Ok(())
    }
}
