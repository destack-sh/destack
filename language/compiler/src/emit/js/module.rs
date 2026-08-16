use crate::{Compiler, CompilerError, CompilerResult};
use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirImported, DirMaterialized,
    DirParsed, Script,
};
use destack_repository::{ArtifactReader, ProfileId, ProviderContext, Target};
use destack_source::ModuleId;

use super::ScriptGenerator;

impl Compiler {
    /// Emit one structured script.
    pub(in crate::emit) fn emit_script(
        &self,
        module_id: ModuleId,
        target: &Target,
        profile: ProfileId,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<Script> {
        // snapshot module for this emit pass
        let module = self.module(context.revision(), module_id)?;
        let parsed = artifacts
            .read::<DirParsed>(module_id)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .read::<DirBound>((module_id, profile))
            .map_err(CompilerError::from)?;
        let imported = artifacts
            .read::<DirImported>((module_id, profile))
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .read::<DirExpanded>((module_id, profile))
            .map_err(CompilerError::from)?;
        let declared = artifacts
            .read::<DirDeclared>((module_id, profile))
            .map_err(CompilerError::from)?;
        let elaborated = artifacts
            .read::<DirElaborated>((module_id, profile))
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .read::<DirChecked>((module_id, profile))
            .map_err(CompilerError::from)?;
        let materialized = artifacts
            .read::<DirMaterialized>((module_id, profile))
            .map_err(CompilerError::from)?;

        // emit one structured script
        let (script, errors) = ScriptGenerator::new(
            module.clone(),
            parsed.clone(),
            bound.clone(),
            imported,
            expanded,
            declared,
            elaborated,
            checked,
            materialized,
            self.repository.string_pool().clone(),
            target,
        )
        .emit()
        .map_err(CompilerError::from)?;

        // emit script diagnostics
        for error in errors {
            self.emit_diagnostic(context, error)?;
        }

        Ok(script)
    }
}
