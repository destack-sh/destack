use destack_artifact::ArtifactPayload;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Build expanded DIR for one module.
    pub(crate) fn provide_dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let _ = self;

        todo!(
            "DIR expansion provider is unavailable for {:?} module {:?} profile {:?}",
            context.artifact_key(),
            module,
            profile,
        )
    }
}
