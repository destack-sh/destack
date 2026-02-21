use destack_dir::{NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::AnalyzeDependencyStage;
use crate::{Compiler, TaskDependencyError};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Provide tree and symbol tables for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_tree_symbols_at_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // remote reads must satisfy the stage gate
        if module_id != module.id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_tree_symbols_unchecked(module, profile, module_id, handle))
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
        // remote reads must satisfy the stage gate
        if module_id != module.id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_tree_symbols_or_local_unchecked(
            module, profile, module_id, tree, symbols, handle,
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

        Ok(self.with_module_tree_symbols_by_id_unchecked(profile, module_id, handle))
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
        // remote reads must satisfy the stage gate
        if module_id != symbols.module_id || module_id != types.module_id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_tree_symbols_types_by_id_unchecked(
            profile, module_id, tree, symbols, types, handle,
        ))
    }

    /// Provide the tree and symbol table for a module in the given profile.
    fn with_module_tree_symbols_unchecked<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> R {
        // use the current module when it matches
        if module_id == module.id {
            let tree = module.dir(profile).tree.read();
            let symbols = module.dir(profile).symbols.read();
            return handle(module, &tree, &symbols);
        }

        // otherwise load the module from the program
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_dir = remote_module.dir(profile);
        let remote_tree = remote_dir.tree.read();
        let remote_symbols = remote_dir.symbols.read();
        handle(&remote_module, &remote_tree, &remote_symbols)
    }

    /// Provide a tree and symbol table by module id.
    fn with_module_tree_symbols_by_id_unchecked<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> R {
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_dir = remote_module.dir(profile);
        let remote_tree = remote_dir.tree.read();
        let remote_symbols = remote_dir.symbols.read();
        handle(&remote_module, &remote_tree, &remote_symbols)
    }

    /// Provide a tree and symbol table, reusing local references when possible.
    fn with_module_tree_symbols_or_local_unchecked<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> R {
        // reuse the provided tables when they match the target module
        if module_id == module.id {
            return handle(module, tree, symbols);
        }

        self.with_module_tree_symbols_unchecked(module, profile, module_id, handle)
    }

    /// Provide tree, symbol, and type tables by module id, reusing local tables when possible.
    fn with_module_tree_symbols_types_by_id_unchecked<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        handle: impl FnOnce(&NodeTree, &SymbolTable, &TypeTable) -> R,
    ) -> R {
        // reuse the provided tables when they match the target module
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
}
