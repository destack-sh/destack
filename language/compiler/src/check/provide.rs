use std::sync::Arc;

use destack_artifact::{ArtifactPayload, DirChecked};
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build checked DIR side tables for one module.
    pub(crate) fn provide_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let _exported = self
            .dir_exported(context, module, profile)
            .map_err(CompilerError::from)?;
        let expanded = self
            .dir_expanded(context, module, profile)
            .map_err(CompilerError::from)?;

        // FUGU #Incomplete: implement proper type checking
        let checked = DirChecked {
            types: Arc::new(dir::TypeSegment::from_base(&expanded.types)),
            layouts: Arc::new(dir::LayoutSegment::new(module)),
            captures: Arc::new(dir::CaptureSegment::new()),
        };

        Ok(ArtifactPayload::DirChecked(checked))
    }
}
