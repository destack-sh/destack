use crate::{Compiler, CompilerError, CompilerResult, GenerateError};
use destack_artifact::ArtifactPayload;
use destack_repository::ProviderContext;

use destack_repository::ProfileId;
use destack_source::{ModuleId, TargetId};

impl Compiler {
    /// Build one module output.
    pub(crate) fn provide_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // resolve target selection
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| GenerateError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let target_name = self.target_name(context.revision(), target)?;
        let resolved_profile = self.profile_id_for_target(context.revision(), module, &target)?;
        if resolved_profile != profile {
            return Err(GenerateError::Internal {
                anchor: module.into(),
                module,
                message: format!(
                    "target '{target_name}' resolved to profile '{resolved_profile:?}', not '{profile:?}'"
                ),
            }
            .into());
        }

        // require selected generation input
        let artifacts = self.artifact_reader(context);
        let input = self.module_output_input(module, profile, &target, &target_config)?;
        artifacts.require(input).map_err(CompilerError::from)?;

        // generate output from the required input
        let output = self.generate_target_module_output(
            module,
            profile,
            &target,
            &target_config,
            &target_name,
            context,
            &artifacts,
        )?;

        Ok(ArtifactPayload::ModuleOutput(output))
    }
}
