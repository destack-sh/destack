use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactSidecar, GlobalEnvironment,
};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{FileContent, ModuleId};

use crate::import::state::ImportState;
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
        for module in self.repository.builtin_package().module_ids() {
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
        let language_modules = self
            .repository
            .builtin_package()
            .module_ids()
            .collect::<Vec<_>>();

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

    /// Collect inputs for the active package index of one profile.
    ///
    /// The index resolves every package's dependency and export declarations,
    /// so it observes each package's config; adding, removing, or editing a
    /// manifest changes the observed set and rebuilds the index.
    pub(crate) fn collect_package_index(
        &self,
        _profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let package_ids = self
            .repository
            .package_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load package ids: {error}"),
            })?;

        // observe every package's config declarations
        let mut dependencies = ArtifactDependencySet::default();
        for package_id in package_ids {
            self.observe_package_config(context, package_id, &mut dependencies)?;
        }

        Ok(dependencies)
    }

    /// Build the active dependency index for one profile.
    pub(crate) fn provide_package_index(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let profile_id = profile;
        let profile = self.profile(context.revision(), profile_id)?;
        let index =
            self.build_package_index(context.revision(), profile_id, profile.conditions())?;

        Ok(ArtifactPayload::PackageIndex(Arc::new(index)))
    }

    /// Collect inputs for imported DIR of one module.
    pub(crate) fn collect_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::package_index(profile));

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
        let profile_id = profile;
        let profile_state = self.profile(context.revision(), profile_id)?;
        let artifacts = self.artifact_reader(context.revision());
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, profile_id)
            .map_err(CompilerError::from)?;
        let package_index = artifacts
            .package_index(profile_id)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module)?;

        // build local module table
        let view = dir::View::new(&parsed.tree);
        let mut state = ImportState::new(
            context.revision(),
            module.as_ref(),
            package_index.as_ref(),
            &profile_state.key,
            profile_state.conditions(),
            self.strings(),
            view,
        );
        self.collect_modules(&mut state, &bound.roots)?;
        let stats = state.stats;
        let (imported, diagnostics) = state.finish();
        context.emit_sidecar(ArtifactSidecar::new(
            "metadata",
            iter::once(("phase", "import")),
            FileContent::Text {
                content: stats.render_metadata(),
            },
        ));
        for diagnostic in diagnostics {
            self.emit_diagnostic(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirImported(Arc::new(imported)))
    }
}
