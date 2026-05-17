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
        let parsed = self
            .dir_parsed(context, module)
            .map_err(CompilerError::from)?;
        let materialized = self
            .dir_materialized(context, module, profile)
            .map_err(CompilerError::from)?;

        // FUGU #Incomplete: implement proper elaboration
        let elaborated = DirElaborated {
            patch: dir::Patch::new(&parsed.tree, "elaborate"),
            bindings: Arc::new(dir::BindingSegment::from_base(&materialized.bindings)),
            types: Arc::new(dir::TypeSegment::from_base(&materialized.types)),
            captures: Arc::new(dir::CaptureSegment::new()),
            layouts: Arc::new(dir::LayoutSegment::new(module)),
            roots: materialized.roots.clone(),
            guards: dir::GuardTable::new(),
        };

        Ok(ArtifactPayload::DirElaborated(elaborated))
    }
}
