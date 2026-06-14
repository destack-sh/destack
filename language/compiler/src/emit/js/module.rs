use crate::{Compiler, CompilerError, CompilerResult};
use destack_artifact::ModuleOutput;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext, Target};
use destack_source::ModuleId;

use super::JsOutputGenerator;

impl Compiler {
    /// Emit one JS module output.
    pub(in crate::emit) fn emit_js_module_output(
        &self,
        module_id: ModuleId,
        target: &Target,
        profile: ProfileId,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<ModuleOutput> {
        // snapshot module for this emit pass
        let module = self.module(context.revision(), module_id)?;
        let parsed = artifacts
            .dir_parsed(module_id)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module_id, profile)
            .map_err(CompilerError::from)?;
        let imported = artifacts
            .dir_imported(module_id, profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module_id, profile)
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .dir_checked(module_id, profile)
            .map_err(CompilerError::from)?;

        // emit one JS output
        let (artifact, errors) = JsOutputGenerator::new(
            module.clone(),
            parsed.clone(),
            bound.clone(),
            imported,
            expanded,
            checked,
            self.repository.string_pool().clone(),
            target,
        )
        .emit()
        .map_err(CompilerError::from)?;

        // emit JS diagnostics
        for error in errors {
            self.emit_diagnostic(context, error)?;
        }

        Ok(ModuleOutput::Js(Box::new(artifact)))
    }
}
