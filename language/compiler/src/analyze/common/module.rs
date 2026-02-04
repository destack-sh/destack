use destack_dir::{Export, NodeTree, StaticKey, SymbolSpace, SymbolTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};
use indexmap::IndexMap;

use crate::Compiler;

// allow staging helpers before full adoption
#[allow(dead_code)]
impl Compiler {
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
