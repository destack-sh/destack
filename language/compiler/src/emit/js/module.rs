use crate::{Compiler, CompilerError, CompilerResult};
use tspp_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirImported, DirMaterialized,
    DirParsed, DirResolved, DirView, Script,
};
use tspp_repository::{ArtifactReader, ProfileId, ProviderContext, Target};
use tspp_source::ModuleId;

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
        let module_id_key = (module_id, profile);
        let view = DirView::materialized(
            artifacts.read::<DirParsed>(module_id)?,
            artifacts.read::<DirBound>(module_id_key)?,
            artifacts.read::<DirImported>(module_id_key)?,
            artifacts.read::<DirExpanded>(module_id_key)?,
            artifacts.read::<DirResolved>(module_id_key)?,
            artifacts.read::<DirDeclared>(module_id_key)?,
            artifacts.read::<DirElaborated>(module_id_key)?,
            artifacts.read::<DirChecked>(module_id_key)?,
            artifacts.read::<DirMaterialized>(module_id_key)?,
        );

        // emit one structured script
        let (script, errors) = ScriptGenerator::new(
            module.clone(),
            view,
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
