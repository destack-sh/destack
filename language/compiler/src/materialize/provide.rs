use std::sync::Arc;

use destack_artifact::{ArtifactPayload, DirMaterialized};
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build materialized DIR for one module.
    pub(crate) fn provide_dir_materialized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let parsed = self
            .dir_parsed(context, module)
            .map_err(CompilerError::from)?;
        let expanded = self
            .dir_expanded(context, module, profile)
            .map_err(CompilerError::from)?;
        let checked = self
            .dir_checked(context, module, profile)
            .map_err(CompilerError::from)?;

        // FUGU #Incomplete: implement proper materialization
        let materialized = DirMaterialized {
            patch: dir::Patch::new(&parsed.tree, "materialize"),
            bindings: Arc::new(dir::BindingSegment::from_base(&expanded.bindings)),
            types: Arc::new(dir::TypeSegment::from_base(&checked.types)),
            captures: Arc::new(dir::CaptureSegment::new()),
            layouts: Arc::new(dir::LayoutSegment::new(module)),
            roots: expanded.roots.clone(),
        };

        Ok(ArtifactPayload::DirMaterialized(materialized))
    }
}
