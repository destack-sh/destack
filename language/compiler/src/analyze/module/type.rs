use destack_dir::TypeTable;
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::AnalyzeDependencyStage;
use crate::{Compiler, TaskDependencyError};

impl Compiler {
    /// Read one module type table, reusing a local table when possible.
    fn with_module_types_read<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        local_types: Option<&TypeTable>,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> R {
        // reuse local table for local reads
        if module_id == module.id {
            if let Some(local_types) = local_types {
                return handle(module, local_types);
            }

            let types = module.dir(profile).types.read();
            return handle(module, &types);
        }

        // otherwise read from the remote module
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let types = remote_module.dir(profile).types.read();
        handle(&remote_module, &types)
    }

    /// Provide type ctx for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_types_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_types_read(module, profile, module_id, None, handle))
    }

    /// Provide type ctx with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_types_or_local_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        types: &TypeTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_types_read(module, profile, module_id, Some(types), handle))
    }

    /// Provide a type table by module id with a stage gate.
    pub(crate) fn with_module_types_by_id_at_stage<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // by-id reads are always cross-module, so always gate
        self.require_module_stage_for_read(module_id, profile, stage)?;

        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(profile).types.read();
        Ok(handle(&remote_module, &remote_types))
    }
}
