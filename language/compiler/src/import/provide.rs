use std::sync::Arc;

use tspp_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, DirBound, DirParsed};
use tspp_dir as dir;
use tspp_repository::{ProfileId, ProviderContext};
use tspp_source::ModuleId;

use crate::import::ImportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for imported DIR of one module.
    pub(crate) fn collect_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));

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
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .read::<DirBound>((module, profile))
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
            profile_state.conditions(),
            context,
            &profile_state.key,
            self.strings(),
            view,
        );
        self.collect_modules(&mut state, &bound.roots)?;
        let stats = state.stats;
        let (imported, diagnostics) = state.finish();
        stats.record(context);
        for diagnostic in diagnostics {
            self.emit_diagnostic(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirImported(Arc::new(imported)))
    }
}
