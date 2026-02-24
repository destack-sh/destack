use destack_dir::{NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::AnalyzeDependencyStage;
use crate::{Compiler, TaskDependencyError};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Read tree and symbol tables for a module, reusing local tables when possible.
    fn with_module_tree_symbols_read<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        local_tree: Option<&NodeTree>,
        local_symbols: Option<&SymbolTable>,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> R {
        // reuse local tables for local reads
        if module_id == module.id {
            if let (Some(local_tree), Some(local_symbols)) = (local_tree, local_symbols) {
                return handle(module, local_tree, local_symbols);
            }

            let tree = module.dir(profile).tree.read();
            let symbols = module.dir(profile).symbols.read();
            return handle(module, &tree, &symbols);
        }

        // otherwise read from the remote module
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_dir = remote_module.dir(profile);
        let tree = remote_dir.tree.read();
        let symbols = remote_dir.symbols.read();
        handle(&remote_module, &tree, &symbols)
    }

    /// Read tree, symbol, and type tables for a module id, reusing local tables when possible.
    fn with_module_tree_symbols_types_by_id_read<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        handle: impl FnOnce(&NodeTree, &SymbolTable, &TypeTable) -> R,
    ) -> R {
        // reuse local tables when all table owners match the target module
        if module_id == symbols.module_id && module_id == types.module_id {
            return handle(tree, symbols, types);
        }

        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_dir = remote_module.dir(profile);
        let remote_tree = remote_dir.tree.read();
        let remote_symbols = remote_dir.symbols.read();
        let remote_types = remote_dir.types.read();
        handle(&remote_tree, &remote_symbols, &remote_types)
    }

    /// Provide tree and symbol tables for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_tree_symbols_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_tree_symbols_read(module, profile, module_id, None, None, handle))
    }

    /// Provide tree and symbol tables with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_tree_symbols_or_local_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(module.id, module_id, profile, stage)?;

        Ok(self.with_module_tree_symbols_read(
            module,
            profile,
            module_id,
            Some(tree),
            Some(symbols),
            handle,
        ))
    }

    /// Provide tree and symbol tables by module id with a stage gate.
    pub(crate) fn with_module_tree_symbols_by_id_at_stage<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // by id reads are always cross-module, so always gate
        self.require_module_stage_for_read(module_id, profile, stage)?;

        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_dir = remote_module.dir(profile);
        let tree = remote_dir.tree.read();
        let symbols = remote_dir.symbols.read();
        Ok(handle(&remote_module, &tree, &symbols))
    }

    /// Provide tree, symbol, and type tables by module id with stage-gated cross-module reads.
    pub(crate) fn with_module_tree_symbols_types_by_id_at_stage<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&NodeTree, &SymbolTable, &TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_stage_for_remote_module_read(symbols.module_id, module_id, profile, stage)?;
        self.require_stage_for_remote_module_read(types.module_id, module_id, profile, stage)?;

        Ok(self.with_module_tree_symbols_types_by_id_read(
            profile, module_id, tree, symbols, types, handle,
        ))
    }
}
