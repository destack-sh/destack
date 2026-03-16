use crate::AnalyzeOptions;
use destack_dir::{InferTable, NodeTree, SymbolTable, TypeTable};
use destack_workspace::{Module, ProfileId};

/// Shared immutable type-resolution view for tree, symbols, and types.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TypeView<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The analyzed syntax tree.
    pub tree: &'a NodeTree,
    /// The symbol table for the active module.
    pub symbols: &'a SymbolTable,
    /// The type table for type-resolution operations.
    pub types: &'a TypeTable,
}

impl<'a> TypeView<'a> {
    /// Construct an immutable type-resolution view.
    pub(crate) fn new(
        module: &'a Module,
        profile: ProfileId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
    ) -> Self {
        Self {
            module,
            profile,
            tree,
            symbols,
            types,
        }
    }

    /// Borrow this type-ctx view as an immutable symbol-and-type view.
    pub(crate) fn symbol_type_view(&self) -> SymbolTypeView<'_> {
        SymbolTypeView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this type view as an immutable module-and-symbol view.
    pub(crate) fn module_symbol_view(&self) -> ModuleSymbolView<'a> {
        ModuleSymbolView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
        }
    }

    /// Borrow this type view as an immutable tree-and-symbol view.
    pub(crate) fn tree_symbol_view(&self) -> TreeSymbolView<'a> {
        TreeSymbolView {
            module: self.module,
            profile: self.profile,
            tree: self.tree,
            symbols: self.symbols,
        }
    }
}

/// Shared immutable module, tree, and symbol view for declaration and lookup operations.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TreeSymbolView<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The analyzed syntax tree.
    pub tree: &'a NodeTree,
    /// The symbol table for the active module.
    pub symbols: &'a SymbolTable,
}

impl<'a> TreeSymbolView<'a> {
    /// Construct an immutable module, tree, and symbol ctx view.
    pub(crate) fn new(
        module: &'a Module,
        profile: ProfileId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
    ) -> Self {
        Self {
            module,
            profile,
            tree,
            symbols,
        }
    }

    /// Borrow this tree-and-symbol view as an immutable module-and-symbol view.
    pub(crate) fn module_symbol_view(&self) -> ModuleSymbolView<'a> {
        ModuleSymbolView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
        }
    }

    /// Borrow this tree-and-symbol view with one explicit type table.
    pub(crate) fn type_view<'b>(&'b self, types: &'b TypeTable) -> TypeView<'b> {
        TypeView {
            module: self.module,
            profile: self.profile,
            tree: self.tree,
            symbols: self.symbols,
            types,
        }
    }
}

/// Shared immutable module-and-symbol view for module-local symbol queries.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleSymbolView<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The symbol table for the active module.
    pub symbols: &'a SymbolTable,
}

impl<'a> ModuleSymbolView<'a> {
    /// Construct an immutable module-and-symbol view.
    pub(crate) fn new(module: &'a Module, profile: ProfileId, symbols: &'a SymbolTable) -> Self {
        Self {
            module,
            profile,
            symbols,
        }
    }
}

/// Shared immutable module-and-type view for module-local type queries.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleTypeView<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The type table for type-resolution operations.
    pub types: &'a TypeTable,
}

impl<'a> ModuleTypeView<'a> {
    /// Construct an immutable module-and-type view.
    pub(crate) fn new(module: &'a Module, profile: ProfileId, types: &'a TypeTable) -> Self {
        Self {
            module,
            profile,
            types,
        }
    }
}

/// Shared immutable symbol-and-type view for module-local type queries.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SymbolTypeView<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The symbol table for the active module.
    pub symbols: &'a SymbolTable,
    /// The type table for type-resolution operations.
    pub types: &'a TypeTable,
}

impl<'a> SymbolTypeView<'a> {
    /// Construct an immutable symbol-and-type ctx view.
    pub(crate) fn new(
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
    ) -> Self {
        Self {
            module,
            profile,
            symbols,
            types,
        }
    }

    /// Borrow this symbol-and-type view as an immutable module-and-symbol view.
    pub(crate) fn module_symbol_view(&self) -> ModuleSymbolView<'_> {
        ModuleSymbolView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
        }
    }

    /// Borrow this symbol-and-type view as an immutable module-and-type view.
    pub(crate) fn module_type_view(&self) -> ModuleTypeView<'_> {
        ModuleTypeView {
            module: self.module,
            profile: self.profile,
            types: self.types,
        }
    }
}

/// Shared mutable commit-phase context.
#[derive(Debug)]
pub(crate) struct CommitContext<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The analyzed syntax tree.
    pub tree: &'a NodeTree,
    /// The symbol table for the active module.
    pub symbols: &'a SymbolTable,
    /// The active analysis options.
    pub options: &'a AnalyzeOptions,
    /// The mutable committed type table.
    pub types: &'a mut TypeTable,
}

impl<'a> CommitContext<'a> {
    /// Construct a commit-phase context.
    pub(crate) fn new(
        module: &'a Module,
        profile: ProfileId,
        options: &'a AnalyzeOptions,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        types: &'a mut TypeTable,
    ) -> Self {
        Self {
            module,
            profile,
            tree,
            symbols,
            options,
            types,
        }
    }

    /// Reborrow this commit context as a type-resolution context.
    pub(crate) fn type_context_reborrow(&mut self) -> TypeContext<'_> {
        TypeContext {
            module: self.module,
            profile: self.profile,
            options: self.options,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }
}

/// Shared mutable assignability-check context.
#[derive(Debug)]
pub(crate) struct AssignContext<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The analyzed syntax tree for assignability helpers.
    pub tree: &'a NodeTree,
    /// The symbol table for relation checks.
    pub symbols: &'a SymbolTable,
    /// The mutable type table for normalization and relation checks.
    pub types: &'a mut TypeTable,
    /// The active analysis options.
    pub options: &'a AnalyzeOptions,
}

impl<'a> AssignContext<'a> {
    /// Reborrow this context for one nested call chain.
    pub(crate) fn reborrow(&mut self) -> AssignContext<'_> {
        AssignContext {
            module: self.module,
            profile: self.profile,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
            options: self.options,
        }
    }

    /// Reborrow this assign context as a type-ctx context.
    pub(crate) fn type_context_reborrow(&mut self) -> TypeContext<'_> {
        TypeContext {
            module: self.module,
            profile: self.profile,
            options: self.options,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this assign context as an immutable symbol-and-type view.
    pub(crate) fn symbol_type_view(&self) -> SymbolTypeView<'_> {
        SymbolTypeView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this assign context as an immutable module-and-symbol view.
    pub(crate) fn module_symbol_view(&self) -> ModuleSymbolView<'_> {
        ModuleSymbolView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
        }
    }
}

/// Shared mutable type-resolution context for tree, symbols, and types.
#[derive(Debug)]
pub(crate) struct TypeContext<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The active analysis options.
    pub options: &'a AnalyzeOptions,
    /// The analyzed syntax tree.
    pub tree: &'a NodeTree,
    /// The symbol table for the active module.
    pub symbols: &'a SymbolTable,
    /// The mutable type table for type-resolution operations.
    pub types: &'a mut TypeTable,
}

impl<'a> TypeContext<'a> {
    /// Construct a type-resolution context for table-driven operations.
    pub(crate) fn new(
        module: &'a Module,
        profile: ProfileId,
        options: &'a AnalyzeOptions,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        types: &'a mut TypeTable,
    ) -> Self {
        Self {
            module,
            profile,
            options,
            tree,
            symbols,
            types,
        }
    }

    /// Return one stable anchor node for diagnostics and synthetic types.
    pub(crate) fn anchor_node(&self) -> destack_dir::LocalNodeIdAny {
        if let Some(primary_declaration) = self
            .symbols
            .active_symbol_ids()
            .find_map(|symbol_id| self.symbols.get_symbol(symbol_id).primary_declaration)
        {
            return primary_declaration.local_id;
        }

        self.tree
            .iter_node_ids()
            .next()
            .expect("expected local analyze tree to contain at least one node")
    }

    /// Reborrow this context for one nested call chain.
    pub(crate) fn reborrow(&mut self) -> TypeContext<'_> {
        TypeContext {
            module: self.module,
            profile: self.profile,
            options: self.options,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this type context as an immutable type view.
    pub(crate) fn type_view(&self) -> TypeView<'_> {
        TypeView {
            module: self.module,
            profile: self.profile,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this type context as an immutable symbol-and-type view.
    pub(crate) fn symbol_type_view(&self) -> SymbolTypeView<'_> {
        SymbolTypeView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this type context as an immutable tree-and-symbol view.
    pub(crate) fn tree_symbol_view(&self) -> TreeSymbolView<'a> {
        TreeSymbolView {
            module: self.module,
            profile: self.profile,
            tree: self.tree,
            symbols: self.symbols,
        }
    }

    /// Borrow this type context as an immutable module-and-symbol view.
    pub(crate) fn module_symbol_view(&self) -> ModuleSymbolView<'a> {
        ModuleSymbolView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
        }
    }

    /// Borrow this type context as an immutable module-and-type view.
    pub(crate) fn module_type_view(&self) -> ModuleTypeView<'_> {
        ModuleTypeView {
            module: self.module,
            profile: self.profile,
            types: self.types,
        }
    }
}

/// Shared mutable infer context for tree, symbols, types, and infer ctx.
#[derive(Debug)]
pub(crate) struct InferContext<'a> {
    /// The module under analysis.
    pub module: &'a Module,
    /// The active profile.
    pub profile: ProfileId,
    /// The active analysis options.
    pub options: &'a AnalyzeOptions,
    /// The analyzed syntax tree.
    pub tree: &'a NodeTree,
    /// The symbol table for the active module.
    pub symbols: &'a SymbolTable,
    /// The mutable type table for infer operations.
    pub types: &'a mut TypeTable,
    /// The mutable infer table for infer operations.
    pub infer: &'a mut InferTable,
}

impl<'a> InferContext<'a> {
    /// Construct an infer context.
    pub(crate) fn new(
        module: &'a Module,
        profile: ProfileId,
        options: &'a AnalyzeOptions,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        types: &'a mut TypeTable,
        infer: &'a mut InferTable,
    ) -> Self {
        Self {
            module,
            profile,
            options,
            tree,
            symbols,
            types,
            infer,
        }
    }

    /// Reborrow this context for one nested call chain.
    pub(crate) fn reborrow(&mut self) -> InferContext<'_> {
        InferContext {
            module: self.module,
            profile: self.profile,
            options: self.options,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
            infer: self.infer,
        }
    }

    /// Reborrow this context as a type-resolution context.
    pub(crate) fn type_context_reborrow(&mut self) -> TypeContext<'_> {
        TypeContext {
            module: self.module,
            profile: self.profile,
            options: self.options,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this infer context as an immutable type-ctx view.
    pub(crate) fn type_view(&self) -> TypeView<'_> {
        TypeView {
            module: self.module,
            profile: self.profile,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this infer context as an immutable symbol-and-type view.
    pub(crate) fn symbol_type_view(&self) -> SymbolTypeView<'_> {
        SymbolTypeView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Borrow this infer context as an immutable tree-and-symbol view.
    pub(crate) fn tree_symbol_view(&self) -> TreeSymbolView<'a> {
        TreeSymbolView {
            module: self.module,
            profile: self.profile,
            tree: self.tree,
            symbols: self.symbols,
        }
    }

    /// Borrow this infer context as an immutable module-and-symbol view.
    pub(crate) fn module_symbol_view(&self) -> ModuleSymbolView<'a> {
        ModuleSymbolView {
            module: self.module,
            profile: self.profile,
            symbols: self.symbols,
        }
    }

    /// Borrow this infer context as an immutable module-and-type view.
    pub(crate) fn module_type_view(&self) -> ModuleTypeView<'_> {
        ModuleTypeView {
            module: self.module,
            profile: self.profile,
            types: self.types,
        }
    }

    /// Split this infer context into type ctx and infer table borrows.
    pub(crate) fn split_type_context_and_infer(&mut self) -> (TypeContext<'_>, &mut InferTable) {
        (
            TypeContext {
                module: self.module,
                profile: self.profile,
                options: self.options,
                tree: self.tree,
                symbols: self.symbols,
                types: self.types,
            },
            self.infer,
        )
    }
}
