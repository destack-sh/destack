use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactPayload, DirMaterialized};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::ModuleId;

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build materialized DIR for one module.
    pub(crate) fn provide_dir_materialized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // require provider inputs
        let artifacts = self.artifact_reader(context);
        artifacts
            .require(ArtifactKey::dir_checked(module, profile))
            .map_err(CompilerError::from)?;

        // load provider inputs
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, profile)
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .dir_checked(module, profile)
            .map_err(CompilerError::from)?;

        // FUGU #Incomplete: implement proper materialization
        let materialized = DirMaterialized {
            patch: dir::Patch::new(&parsed.tree, "materialize"),
            bindings: Arc::new(dir::BindingSegment::from_base(&expanded.bindings)),
            types: Arc::new(dir::TypeSegment::from_base(&checked.types)),
            statics: Arc::new(dir::StaticSegment::from_base(&checked.statics)),
            resolutions: Arc::new(dir::ResolutionSegment::new(module)),
            generics: Arc::new(dir::GenericSegment::from_base(&checked.generics)),
            relations: Arc::new(dir::RelationSegment::new(module)),
            coercions: Arc::new(dir::CoercionSegment::new(module)),
            captures: Arc::new(dir::CaptureSegment::new(module)),
            layouts: Arc::new(dir::LayoutSegment::from_base(&checked.layouts)),
            roots: expanded.roots.clone(),
        };

        Ok(ArtifactPayload::DirMaterialized(materialized))
    }
}
