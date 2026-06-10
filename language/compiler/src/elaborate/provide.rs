use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactPayload, DirElaborated};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::ModuleId;

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build elaborated DIR for one checked module.
    pub(crate) fn provide_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // require provider inputs
        let artifacts = self.artifact_reader(context);
        artifacts
            .require(ArtifactKey::dir_materialized(module, profile))
            .map_err(CompilerError::from)?;

        // load provider inputs
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let materialized = artifacts
            .dir_materialized(module, profile)
            .map_err(CompilerError::from)?;

        // FUGU #Incomplete: implement proper elaboration
        let elaborated = DirElaborated {
            patch: dir::Patch::new(&parsed.tree, "elaborate"),
            bindings: Arc::new(dir::BindingSegment::from_base(&materialized.bindings)),
            types: Arc::new(dir::TypeSegment::from_base(&materialized.types)),
            statics: Arc::new(dir::StaticSegment::from_base(&materialized.statics)),
            resolutions: Arc::new(dir::ResolutionSegment::new(module)),
            generics: Arc::new(dir::GenericSegment::from_base(&materialized.generics)),
            relations: Arc::new(dir::RelationSegment::new(module)),
            coercions: Arc::new(dir::CoercionSegment::new(module)),
            captures: Arc::new(dir::CaptureSegment::new(module)),
            layouts: Arc::new(dir::LayoutSegment::from_base(&materialized.layouts)),
            roots: materialized.roots.clone(),
            guards: dir::GuardTable::new(module),
        };

        Ok(ArtifactPayload::DirElaborated(elaborated))
    }
}
