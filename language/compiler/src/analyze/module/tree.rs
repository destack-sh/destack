use destack_dir::{NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::DirReadBoundary;
use crate::analyze::common::TreeSymbolView;
use crate::{BuildRequirementError, Compiler};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Read tree and symbol ctx for a module, reusing local ctx when possible.
    fn with_module_tree_symbols_read<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        local_tree: Option<&NodeTree>,
        local_symbols: Option<&SymbolTable>,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        // reuse local ctx for local reads
        if module_id == module.id {
            if let (Some(local_tree), Some(local_symbols)) = (local_tree, local_symbols) {
                return Ok(handle(module, local_tree, local_symbols));
            }

            let tree = module.dir(profile).tree.read();
            let symbols = module.dir(profile).symbols.read();
            return Ok(handle(module, &tree, &symbols));
        }

        // otherwise read from the remote module
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let snapshot = self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
        Ok(handle(&remote_module, &snapshot.tree, &snapshot.symbols))
    }

    /// Read tree, symbol, and type ctx for a module id, reusing local ctx when possible.
    fn with_module_tree_symbols_types_by_id_read<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&NodeTree, &SymbolTable, &TypeTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        // reuse local ctx when all table owners match the target module
        if module_id == symbols.module_id && module_id == types.module_id {
            return Ok(handle(tree, symbols, types));
        }

        let remote_module = self.program.modules.get(module_id);
        let _remote_module = remote_module.read();
        let snapshot = self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
        Ok(handle(&snapshot.tree, &snapshot.symbols, &snapshot.types))
    }

    /// Provide tree and symbol ctx for a module with boundary-gated cross-module reads.
    pub(crate) fn with_module_tree_symbols_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(module.id, module_id, profile, boundary)?;

        self.with_module_tree_symbols_read(module, profile, module_id, None, None, boundary, handle)
    }

    /// Provide tree and symbol ctx with boundary-gated cross-module reads and local reuse.
    pub(crate) fn with_module_tree_symbols_or_local_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(module.id, module_id, profile, boundary)?;

        self.with_module_tree_symbols_read(
            module,
            profile,
            module_id,
            Some(tree),
            Some(symbols),
            boundary,
            handle,
        )
    }

    /// Provide one tree-symbol view with boundary-gated cross-module reads.
    pub(crate) fn with_module_tree_symbol_view_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        boundary: DirReadBoundary,
        handle: impl FnOnce(TreeSymbolView<'_>) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.with_module_tree_symbols_at_boundary(
            module,
            profile,
            module_id,
            boundary,
            |module, tree, symbols| handle(TreeSymbolView::new(module, profile, tree, symbols)),
        )
    }

    /// Provide one tree-symbol view with boundary-gated cross-module reads and local reuse.
    pub(crate) fn with_module_tree_symbol_view_or_local_at_boundary<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        boundary: DirReadBoundary,
        handle: impl FnOnce(TreeSymbolView<'_>) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.with_module_tree_symbols_or_local_at_boundary(
            module,
            profile,
            module_id,
            tree,
            symbols,
            boundary,
            |module, tree, symbols| handle(TreeSymbolView::new(module, profile, tree, symbols)),
        )
    }

    /// Provide tree and symbol ctx by module id with a boundary gate.
    pub(crate) fn with_module_tree_symbols_by_id_at_boundary<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        // by id reads are always cross-module, so always gate
        self.require_module_boundary_for_read(module_id, profile, boundary)?;

        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let snapshot = self.require_artifact_dir_for_boundary(module_id, profile, boundary)?;
        Ok(handle(&remote_module, &snapshot.tree, &snapshot.symbols))
    }

    /// Provide tree, symbol, and type ctx by module id with boundary-gated cross-module reads.
    pub(crate) fn with_module_tree_symbols_types_by_id_at_boundary<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        boundary: DirReadBoundary,
        handle: impl FnOnce(&NodeTree, &SymbolTable, &TypeTable) -> R,
    ) -> Result<R, BuildRequirementError> {
        self.require_boundary_for_remote_module_read(
            symbols.module_id,
            module_id,
            profile,
            boundary,
        )?;
        self.require_boundary_for_remote_module_read(
            types.module_id,
            module_id,
            profile,
            boundary,
        )?;

        self.with_module_tree_symbols_types_by_id_read(
            profile, module_id, tree, symbols, types, boundary, handle,
        )
    }
}
