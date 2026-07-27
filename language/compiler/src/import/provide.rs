use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactSidecar, GlobalEnvironment,
    PackageGraphProjection,
};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{Content, ModuleId};

use crate::import::ImportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for the global environment of one profile.
    pub(crate) fn collect_global_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();

        // language item modules back the language environment
        for module in self.repository.builtin_module_ids(context.revision())? {
            dependencies.require(ArtifactKey::dir_bound(module, profile));
        }

        // global module export surfaces back the global targets
        let globals = self.load_global_module_ids(profile, context)?;
        let artifacts = self.artifact_reader(context.revision());
        self.collect_exported_modules(globals, profile, &artifacts, &mut dependencies)?;

        Ok(dependencies)
    }

    /// Build the global environment for one profile.
    pub(crate) fn provide_global_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // discover environment inputs
        let globals = self.load_global_module_ids(profile, context)?;
        let language_modules = self.repository.builtin_module_ids(context.revision())?;

        // build language environment for profile
        let artifacts = self.artifact_reader(context.revision());
        let language = self.build_language_environment(profile, &artifacts, &language_modules)?;
        let global_targets = self.build_global_targets(profile, &artifacts, &globals)?;
        let environment = GlobalEnvironment {
            language,
            globals,
            global_targets_by_key: global_targets,
        };

        Ok(ArtifactPayload::GlobalEnvironment(Arc::new(environment)))
    }

    /// Collect inputs for the active package graph of one profile.
    ///
    /// The graph resolves every package's active dependency and export declarations.
    /// It observes the complete package set, every package configuration, and the module set.
    pub(crate) fn collect_package_graph(
        &self,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let package_ids = self
            .repository
            .package_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load package ids: {error}"),
            })?;

        // observe the exact package set and every package declaration
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.observe_packages(&package_ids);
        for package_id in package_ids {
            self.observe_package_config(context, package_id, &mut dependencies)?;
        }

        // observe the exact module set used by import specifiers
        let modules = self.repository.module_ids(context.revision())?;
        dependencies.observe_modules(&modules);

        Ok(dependencies)
    }

    /// Build the active package graph for one profile.
    pub(crate) fn provide_package_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let profile_state = self.profile(context.revision(), profile)?;
        let graph =
            self.build_package_graph(context.revision(), profile, profile_state.conditions())?;

        Ok(ArtifactPayload::PackageGraph(Arc::new(graph)))
    }

    /// Collect inputs for imported DIR of one module.
    pub(crate) fn collect_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.project(
            ArtifactKey::package_graph(profile),
            PackageGraphProjection::Nodes,
        );

        Ok(dependencies)
    }

    /// Build imported DIR for one module.
    pub(crate) fn provide_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let profile_state = self.profile(context.revision(), profile)?;
        let artifacts = self.artifact_reader(context.revision());
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, profile)
            .map_err(CompilerError::from)?;
        let package_graph = artifacts
            .package_graph(profile)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module)?;
        let package = self.package(context.revision(), module.package_id)?;
        let environment = self.environment(context.revision())?;

        // build local module table
        let view = dir::View::new(&parsed.tree);
        let mut state = ImportState::new(
            context.revision(),
            module.as_ref(),
            package.as_ref(),
            environment.as_ref(),
            package_graph.as_ref(),
            &profile_state.key,
            self.strings(),
            view,
        );
        self.collect_modules(&mut state, &bound.roots)?;
        let stats = state.stats;
        let (imported, diagnostics) = state.finish();
        context.emit_sidecar(ArtifactSidecar::new(
            "metadata",
            iter::once(("phase", "import")),
            Content::Text {
                content: stats.render_metadata(),
            },
        ));
        for diagnostic in diagnostics {
            self.emit_diagnostic(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirImported(Arc::new(imported)))
    }
}
