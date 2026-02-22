use destack_dir::TypeTable;
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::AnalyzeDependencyStage;
use crate::{Compiler, TaskDependencyError};

impl Compiler {
    /// Provide type tables for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_types_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_types_unchecked(module, profile, module_id, handle))
    }

    /// Provide type tables with stage-gated cross-module reads and local reuse.
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

        Ok(self.with_module_types_or_local_unchecked(module, profile, module_id, types, handle))
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

    /// Provide mutable type tables with stage-gated cross-module reads.
    pub(crate) fn with_module_types_mut_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &mut TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_types_mut_unchecked(module, profile, module_id, handle))
    }

    /// Provide the type table for a module in the given profile.
    fn with_module_types_unchecked<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> R {
        // use the current module when it matches
        if module_id == module.id {
            let types = module.dir(profile).types.read();
            return handle(module, &types);
        }

        // otherwise load the module from the program
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(profile).types.read();
        handle(&remote_module, &remote_types)
    }

    /// Provide a type table, reusing local references when possible.
    fn with_module_types_or_local_unchecked<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        types: &TypeTable,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> R {
        // reuse the provided table when it matches the target module
        if module_id == module.id {
            return handle(module, types);
        }

        self.with_module_types_unchecked(module, profile, module_id, handle)
    }

    /// Provide the type table for a module in the given profile, with mutable access.
    fn with_module_types_mut_unchecked<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &mut TypeTable) -> R,
    ) -> R {
        // use the current module when it matches
        if module_id == module.id {
            let mut types = module.dir(profile).types.write();
            return handle(module, &mut types);
        }

        // otherwise load the module from the program
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let mut remote_types = remote_module.dir(profile).types.write();
        handle(&remote_module, &mut remote_types)
    }
}
