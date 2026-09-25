use std::sync::Arc;

use tspp_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, DirBound, DirExpanded, DirParsed,
};
use tspp_dir as dir;
use tspp_repository::{ProfileId, ProviderContext};
use tspp_source::ModuleId;

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for expanded DIR of one module.
    pub(crate) fn collect_dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_imported(module, profile));

        Ok(dependencies)
    }

    /// Build expanded DIR for one module.
    pub(crate) fn provide_dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .read::<DirBound>((module, profile))
            .map_err(CompilerError::from)?;

        // TODO #Incomplete: implement proper expansion
        let expanded = DirExpanded {
            patch: dir::Patch::new(&parsed.tree, "expand"),
            bindings: Arc::new(dir::BindingSegment::from_base(&bound.bindings)),
            modules: Arc::new(dir::ModuleSegment::new(module)),
            types: Arc::new(dir::TypeSegment::from_base(&bound.types)),
            statics: Arc::new(dir::StaticSegment::from_base(&bound.statics)),
            macros: dir::MacroTable::new(module),
            roots: bound.roots.clone(),
        };

        Ok(ArtifactPayload::DirExpanded(Arc::new(expanded)))
    }
}
