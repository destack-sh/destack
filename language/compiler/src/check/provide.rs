use destack_artifact::ArtifactPayload;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::check::CheckState;
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

        // load provider inputs
        let expanded = artifacts
            .dir_expanded(module, profile)
            .map_err(CompilerError::from)?;

        // visit DIR, solve constraints, then validate obligations
        let mut state = CheckState::new(module, profile, expanded.as_ref());
        self.visit_check(&mut state).map_err(CompilerError::from)?;
        self.solve_check(&mut state).map_err(CompilerError::from)?;
        self.validate_check(&mut state)
            .map_err(CompilerError::from)?;

        Ok(ArtifactPayload::DirChecked(state.finish()))
    }
}
