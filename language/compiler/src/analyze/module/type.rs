use destack_dir::TypeTable;
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::DirReadBoundary;
use crate::{BuildRequirementError, Compiler};

impl Compiler {
    /// Read one module type table, reusing a local table when possible.
    fn with_module_types_read<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        local_types: Option<&TypeTable>,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        // reuse local table for local reads
        if module_id == module.id {
            if let Some(local_types) = local_types {
                return Ok(handle(module, local_types));
            }

            let types = module.dir(profile).types.read();
            return Ok(handle(module, &types));
        }

        // otherwise read from the remote module
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let snapshot = self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
        Ok(handle(&remote_module, &snapshot.types))
    }

    /// Provide type ctx for a module with boundary-gated cross-module reads.
    pub(crate) fn with_module_types_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(module.id, module_id, profile, boundary)?;

        self.with_module_types_read(module, profile, module_id, None, boundary, handle)
    }

    /// Provide type ctx with boundary-gated cross-module reads and local reuse.
    pub(crate) fn with_module_types_or_local_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        types: &TypeTable,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(module.id, module_id, profile, boundary)?;

        self.with_module_types_read(module, profile, module_id, Some(types), boundary, handle)
    }

    /// Provide a type table by module id with a boundary gate.
    pub(crate) fn with_module_types_by_id_at_boundary<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        // by-id reads are always cross-module, so always gate
        self.require_module_boundary_for_read(module_id, profile, boundary)?;

        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let snapshot = self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
        Ok(handle(&remote_module, &snapshot.types))
    }
}
