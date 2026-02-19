use destack_dir::{Export, NodeTree, StaticKey, SymbolSpace, SymbolTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};
use indexmap::IndexMap;

use crate::{Compiler, TaskDependencyError};

/// Stage contract for cross-module analyze table reads.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalyzeDependencyStage {
    /// Read data owned by declare.
    Declare,
    /// Read data owned by export.
    Export,
    /// Read data owned by infer.
    Infer,
    /// Read data owned by validate.
    Validate,
}

// allow staging helpers before full adoption
#[allow(dead_code, clippy::too_many_arguments)]
impl Compiler {
    /// Ensure a module has completed the stage required for one cross-module read.
    fn require_module_stage_for_read(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        stage: AnalyzeDependencyStage,
    ) -> Result<(), TaskDependencyError> {
        // gate reads by stage ownership
        match stage {
            AnalyzeDependencyStage::Declare => {
                self.require_analyze_module_declare(module_id, profile)
            }
            AnalyzeDependencyStage::Export => {
                self.require_analyze_module_export(module_id, profile)
            }
            AnalyzeDependencyStage::Infer => self.require_analyze_module_infer(module_id, profile),
            AnalyzeDependencyStage::Validate => {
                self.require_analyze_module_validate(module_id, profile)
            }
        }
    }

    /// Provide the symbol table for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_symbols_for_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // remote reads must satisfy the stage gate
        if module_id != module.id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_symbols(module, profile, module_id, handle))
    }

    /// Provide symbol tables with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_symbols_or_local_for_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // remote reads must satisfy the stage gate
        if module_id != module.id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_symbols_or_local(module, profile, module_id, symbols, handle))
    }

    /// Provide tree and symbol tables for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_tree_symbols_for_stage<R>(
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

        Ok(self.with_module_tree_symbols(module, profile, module_id, handle))
    }

    /// Provide tree and symbol tables with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_tree_symbols_or_local_for_stage<R>(
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

        Ok(self
            .with_module_tree_symbols_or_local(module, profile, module_id, tree, symbols, handle))
    }

    /// Provide type tables for a module with stage-gated cross-module reads.
    pub(crate) fn with_module_types_for_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // remote reads must satisfy the stage gate
        if module_id != module.id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_types(module, profile, module_id, handle))
    }

    /// Provide type tables with stage-gated cross-module reads and local reuse.
    pub(crate) fn with_module_types_or_local_for_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        types: &TypeTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // remote reads must satisfy the stage gate
        if module_id != module.id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_types_or_local(module, profile, module_id, types, handle))
    }

    /// Provide a type table by module id with a stage gate.
    pub(crate) fn with_module_types_by_id_for_stage<R>(
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

    /// Provide the symbol table for a module in the given profile.
    pub(crate) fn with_module_symbols<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // use the current module when it matches
        if module_id == module.id {
            let symbols = module.dir(profile).symbols.read();
            return handle(module, &symbols);
        }

        // otherwise load the module from the program
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        handle(&remote_module, &remote_symbols)
    }

    /// Provide a symbol table, reusing local references when possible.
    pub(crate) fn with_module_symbols_or_local<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // reuse the provided table when it matches the target module
        if module_id == module.id {
            return handle(module, symbols);
        }

        self.with_module_symbols(module, profile, module_id, handle)
    }

    /// Provide the symbol table from the base directory for a module.
    pub(crate) fn with_module_symbols_base<R>(
        &self,
        module: &Module,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // use the current module when it matches
        if module_id == module.id {
            let symbols = module.dir_base().symbols.read();
            return handle(module, &symbols);
        }

        // otherwise load the module from the program
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir_base().symbols.read();
        handle(&remote_module, &remote_symbols)
    }

    /// Provide a base symbol table, reusing local references when possible.
    pub(crate) fn with_module_symbols_base_or_local<R>(
        &self,
        module: &Module,
        module_id: ModuleId,
        symbols: &SymbolTable,
        handle: impl FnOnce(&Module, &SymbolTable) -> R,
    ) -> R {
        // reuse the provided table when it matches the target module
        if module_id == module.id {
            return handle(module, symbols);
        }

        self.with_module_symbols_base(module, module_id, handle)
    }

    /// Provide the tree and symbol table for a module in the given profile.
    pub(crate) fn with_module_tree_symbols<R>(
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
    pub(crate) fn with_module_tree_symbols_by_id<R>(
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

    /// Provide tree and symbol tables by module id after resolve dependencies are ready.
    pub(crate) fn with_module_tree_symbols_by_id_for_resolve<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        self.require_resolve_module_direct(module_id, profile)?;
        Ok(self.with_module_tree_symbols_by_id(profile, module_id, handle))
    }

    /// Provide tree and symbol tables by module id with a stage gate.
    pub(crate) fn with_module_tree_symbols_by_id_for_stage<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // by-id reads are always cross-module, so always gate
        self.require_module_stage_for_read(module_id, profile, stage)?;

        Ok(self.with_module_tree_symbols_by_id(profile, module_id, handle))
    }

    /// Provide a tree and symbol table, reusing local references when possible.
    pub(crate) fn with_module_tree_symbols_or_local<R>(
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

        self.with_module_tree_symbols(module, profile, module_id, handle)
    }

    /// Provide the type table for a module in the given profile.
    pub(crate) fn with_module_types<R>(
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
    pub(crate) fn with_module_types_or_local<R>(
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

        self.with_module_types(module, profile, module_id, handle)
    }

    /// Provide the type table for a module in the given profile, with mutable access.
    pub(crate) fn with_module_types_mut<R>(
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

    /// Provide mutable type tables with stage-gated cross-module reads.
    pub(crate) fn with_module_types_mut_for_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &mut TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // remote reads must satisfy the stage gate
        if module_id != module.id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_types_mut(module, profile, module_id, handle))
    }

    /// Provide a mutable type table, reusing local references when possible.
    pub(crate) fn with_module_types_mut_or_local<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        types: &mut TypeTable,
        handle: impl FnOnce(&Module, &mut TypeTable) -> R,
    ) -> R {
        // reuse the provided table when it matches the target module
        if module_id == module.id {
            return handle(module, types);
        }

        self.with_module_types_mut(module, profile, module_id, handle)
    }

    /// Provide mutable type tables with stage-gated reads and local reuse.
    pub(crate) fn with_module_types_mut_or_local_for_stage<R>(
        &self,
        module: &Module,
        profile: ProfileId,
        module_id: ModuleId,
        types: &mut TypeTable,
        stage: AnalyzeDependencyStage,
        handle: impl FnOnce(&Module, &mut TypeTable) -> R,
    ) -> Result<R, TaskDependencyError> {
        // remote reads must satisfy the stage gate
        if module_id != module.id {
            self.require_module_stage_for_read(module_id, profile, stage)?;
        }

        Ok(self.with_module_types_mut_or_local(module, profile, module_id, types, handle))
    }

    /// Provide exported symbols for a module in the given profile.
    pub(crate) fn with_module_exports<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        handle: impl FnOnce(&Module, &IndexMap<(SymbolSpace, StaticKey), Export>) -> R,
    ) -> R {
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_exports = remote_module.dir(profile).exported_symbols.read();
        handle(&remote_module, &remote_exports)
    }

    /// Provide a symbol table by module id, reusing local symbols when possible.
    pub(crate) fn with_module_symbols_by_id<R>(
        &self,
        profile: ProfileId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        handle: impl FnOnce(&SymbolTable) -> R,
    ) -> R {
        // reuse the provided table when it matches the target module
        if module_id == symbols.module_id {
            return handle(symbols);
        }

        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        handle(&remote_symbols)
    }

    /// Provide tree, symbol, and type tables by module id, reusing local tables when possible.
    pub(crate) fn with_module_tree_symbols_types_by_id<R>(
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
