use destack_artifact::ArtifactPayload;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::check::CheckState;
use crate::{Compiler, CompilerResult};

impl Compiler {
    /// Build checked DIR side tables for one module.
    pub(crate) fn provide_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let mut check = CheckState::new(self, context, profile);
        check.load(module)?;
        check.walk()?;
        check.solve()?;
        check.validate()?;
        let checked = check.finish(module)?;

        Ok(ArtifactPayload::DirChecked(checked))
    }
}
