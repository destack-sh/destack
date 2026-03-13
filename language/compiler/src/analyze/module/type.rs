use destack_dir::{GlobalSymbolId, SymbolType, Type, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::DirReadBoundary;
use crate::analyze::common::TypeContext;
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

            if let Some(dir) = self.current_active_dir_frame(module_id, profile, boundary) {
                let types = dir.types.read();
                return Ok(handle(module, &types));
            }

            let snapshot = self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
            return Ok(handle(module, &snapshot.types));
        }

        // otherwise read from the remote module
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        if let Some(dir) = self.current_active_dir_frame(module_id, profile, boundary) {
            let types = dir.types.read();
            return Ok(handle(&remote_module, &types));
        }

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
        if let Some(dir) = self.current_active_dir_frame(module_id, profile, boundary) {
            let types = dir.types.read();
            return Ok(handle(&remote_module, &types));
        }

        let snapshot = self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
        Ok(handle(&remote_module, &snapshot.types))
    }

    /// Read one committed remote alias target snapshot at a boundary.
    pub(crate) fn remote_alias_target_snapshot_at_boundary(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        boundary: DirReadBoundary,
    ) -> Result<
        (
            Option<GlobalSymbolId>,
            Option<(GlobalSymbolId, Type, TypeTable)>,
            Option<GlobalSymbolId>,
        ),
        BuildRequirementError,
    > {
        self.require_boundary_for_remote_module_read(
            module.id,
            symbol.module_id,
            profile,
            boundary,
        )?;

        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let snapshot =
            self.require_artifact_dir_for_boundary(symbol.module_id, profile, boundary)?;
        let symbol_entry = snapshot.symbols.get_symbol(symbol.local_id);
        if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
            let target_symbol = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
            return Ok((None, None, target_symbol));
        }

        let typed_symbol =
            GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(symbol_entry.ty));
        let Some(remote_target_id) = snapshot.types.get_alias_target_type_id(typed_symbol) else {
            let target_symbol = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
            return Ok((Some(typed_symbol), None, target_symbol));
        };

        let mut remote_snapshot = snapshot.types.clone();
        if matches!(
            remote_snapshot.get_type(remote_target_id),
            Type::Unevaluated(_)
        ) {
            let options = self.analyze_context_options_for_module(remote_module.id);
            let mut ctx = TypeContext::new(
                &remote_module,
                profile,
                &options,
                &snapshot.tree,
                &snapshot.symbols,
                &mut remote_snapshot,
            );

            if let Err(error) = self.resolve_declared_type(&mut ctx, remote_target_id) {
                self.error(error);
            }
        }

        if matches!(
            remote_snapshot.get_type(remote_target_id),
            Type::Unevaluated(_)
        ) {
            let target_symbol = symbol_entry.target_symbol.or(symbol_entry.canonical_symbol);
            return Ok((Some(typed_symbol), None, target_symbol));
        }

        let remote_target_ty = remote_snapshot.get_type(remote_target_id).clone();
        Ok((
            Some(typed_symbol),
            Some((typed_symbol, remote_target_ty, remote_snapshot)),
            None,
        ))
    }
}
