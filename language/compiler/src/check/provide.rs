use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactPayload, DirChecked};
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
        let artifacts = self.artifact_reader(context);

        // require prior phase completion
        artifacts
            .require(ArtifactKey::dir_exported(module, profile))
            .map_err(CompilerError::from)?;

        // load provider inputs
        let expanded = artifacts
            .dir_expanded(module, profile)
            .map_err(CompilerError::from)?;

        // FUGU #Incomplete: implement proper type checking
        let checked = DirChecked {
            types: Arc::new(dir::TypeSegment::from_base(&expanded.types)),
            resolutions: Arc::new(dir::ResolutionSegment::new(module)),
            instances: Arc::new(dir::InstanceSegment::new(module)),
            relations: Arc::new(dir::RelationSegment::new(module)),
            layouts: Arc::new(dir::LayoutSegment::new(module)),
            captures: Arc::new(dir::CaptureSegment::new(module)),
        };

        Ok(ArtifactPayload::DirChecked(checked))
    }
}
