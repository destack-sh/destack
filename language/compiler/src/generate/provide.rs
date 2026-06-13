use crate::{Compiler, CompilerResult, GenerateError};
use destack_artifact::{ArtifactDependencySet, ArtifactPayload};
use destack_repository::ProviderContext;
use std::sync::Arc;

use destack_repository::ProfileId;
use destack_source::{ModuleId, TargetId};

impl Compiler {
    /// Collect inputs for one module output.
    pub(crate) fn collect_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // the generation input depends on the resolved target pipeline
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| GenerateError::Internal {
                    anchor: module.into(),
                    module,
                    message: format!("target '{target}' not found"),
                })?;
        let input = self.module_output_input(module, profile, &target, &target_config)?;

        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(input);
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

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

        // generate output from the declared input
        let artifacts = self.artifact_reader(context.revision());
        let output = self.generate_target_module_output(
            module,
            profile,
            &target,
            &target_config,
            &target_name,
            context,
            &artifacts,
        )?;

        Ok(ArtifactPayload::ModuleOutput(Arc::new(output)))
    }
}
