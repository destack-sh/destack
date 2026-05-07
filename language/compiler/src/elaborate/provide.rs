use destack_artifact::{ArtifactKey, ArtifactPayload};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Build elaborated DIR for one checked module.
    pub(crate) fn provide_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifact_key = ArtifactKey::dir_elaborated(module, profile);
        assert_eq!(
            artifact_key,
            context.artifact_key(),
            "compiler attempted to provide the wrong artifact"
        );

        todo!("DIR elaboration provider is unavailable until patch-backed output is wired")
    }
}
