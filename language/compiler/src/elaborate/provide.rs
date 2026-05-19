use std::sync::Arc;

use destack_artifact::{ArtifactPayload, DirElaborated};
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build elaborated DIR for one checked module.
    pub(crate) fn provide_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let materialized = artifacts
            .dir_materialized(module, profile)
            .map_err(CompilerError::from)?;

        // FUGU #Incomplete: implement proper elaboration
        let elaborated = DirElaborated {
            patch: dir::Patch::new(&parsed.tree, "elaborate"),
            bindings: Arc::new(dir::BindingSegment::from_base(&materialized.bindings)),
            types: Arc::new(dir::TypeSegment::from_base(&materialized.types)),
            resolutions: Arc::new(dir::ResolutionSegment::new(module)),
            instances: Arc::new(dir::InstanceSegment::from_base(&materialized.instances)),
            relations: Arc::new(dir::RelationSegment::new(module)),
            captures: Arc::new(dir::CaptureSegment::new(module)),
            layouts: Arc::new(dir::LayoutSegment::from_base(&materialized.layouts)),
            roots: materialized.roots.clone(),
            guards: dir::GuardTable::new(module),
        };

        Ok(ArtifactPayload::DirElaborated(elaborated))
    }
}
