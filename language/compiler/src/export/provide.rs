use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, DirBound, DirExpanded, DirImported,
    DirParsed,
};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::ModuleId;

use crate::export::state::ExportState;
use crate::{Compiler, CompilerError, CompilerResult, ExportError};

impl Compiler {
    /// Collect inputs for exported DIR of one module.
    pub(crate) fn collect_dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_imported(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));

        Ok(dependencies)
    }

    /// Build exported DIR for one module.
    pub(crate) fn provide_dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let profile_id = profile;
        let profile_state = self.profile(context.revision(), profile_id)?;
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .read::<DirBound>((module, profile_id))
            .map_err(CompilerError::from)?;
        let imported = artifacts
            .read::<DirImported>((module, profile_id))
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .read::<DirExpanded>((module, profile_id))
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module)?;
        let package = self.package(context.revision(), module.package_id)?;
        let environment = self.environment(context.revision())?;

        // build expanded export inputs
        let view = expanded.view(&parsed);
        let bindings = expanded.binding_table(&bound);
        let modules = expanded.module_table(&imported);
        let mut state = ExportState::new(
            view,
            module.as_ref(),
            package.as_ref(),
            environment.as_ref(),
            &profile_state.key,
            bound.namespace_scope,
            bindings,
            modules,
            self.strings(),
        );
        self.collect_exports(&mut state, &expanded.roots)
            .map_err(CompilerError::from)?;
        let stats = state.stats;
        let (exported, diagnostics) = state.finish();
        stats.record(context);
        for diagnostic in diagnostics {
            self.emit_diagnostic::<ExportError>(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirExported(Arc::new(exported)))
    }
}
