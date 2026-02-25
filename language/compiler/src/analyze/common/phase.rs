use crate::AnalyzeOptions;
use destack_dir::{InferTable, NodeTree, SymbolTable, TypeTable};
use destack_workspace::{Module, ProfileId};

/// Shared immutable module-level analyze inputs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleContext<'a> {
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
}

impl<'a> ModuleContext<'a> {
    /// Construct a module context from module-local analyze inputs.
    pub(crate) fn new(
        module: &'a Module,
        profile: ProfileId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        options: &'a AnalyzeOptions,
    ) -> Self {
        Self {
            module,
            profile,
            tree,
            symbols,
            options,
        }
    }
}

/// Shared mutable commit-phase context.
#[derive(Debug)]
pub(crate) struct CommitContext<'a> {
    /// The immutable module-level context.
    pub module: ModuleContext<'a>,
    /// The mutable committed type table.
    pub types: &'a mut TypeTable,
}

impl<'a> CommitContext<'a> {
    /// Construct a commit-phase context.
    pub(crate) fn new(module: ModuleContext<'a>, types: &'a mut TypeTable) -> Self {
        Self { module, types }
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
    /// Construct an assignability context.
    pub(crate) fn new(
        module: &'a Module,
        profile: ProfileId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        types: &'a mut TypeTable,
        options: &'a AnalyzeOptions,
    ) -> Self {
        Self {
            module,
            profile,
            tree,
            symbols,
            types,
            options,
        }
    }

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

    /// Reborrow this assign context as a type-tables context.
    pub(crate) fn type_tables_reborrow(&mut self) -> TypeTablesContext<'_> {
        TypeTablesContext {
            module: self.module,
            profile: self.profile,
            options: self.options,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }
}

/// Shared mutable type-resolution context for tree, symbols, and types.
#[derive(Debug)]
pub(crate) struct TypeTablesContext<'a> {
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

impl<'a> TypeTablesContext<'a> {
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

    /// Reborrow this context for one nested call chain.
    pub(crate) fn reborrow(&mut self) -> TypeTablesContext<'_> {
        TypeTablesContext {
            module: self.module,
            profile: self.profile,
            options: self.options,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Reborrow this context for one module-local symbol and tree view.
    pub(crate) fn reborrow_for_module<'b>(
        &'b mut self,
        module: &'b Module,
        tree: &'b NodeTree,
        symbols: &'b SymbolTable,
    ) -> TypeTablesContext<'b> {
        TypeTablesContext {
            module,
            profile: self.profile,
            options: self.options,
            tree,
            symbols,
            types: self.types,
        }
    }

    /// Reborrow this context for one module-local view and one explicit options set.
    pub(crate) fn reborrow_for_module_with_options<'b>(
        &'b mut self,
        module: &'b Module,
        options: &'b AnalyzeOptions,
        tree: &'b NodeTree,
        symbols: &'b SymbolTable,
    ) -> TypeTablesContext<'b> {
        TypeTablesContext {
            module,
            profile: self.profile,
            options,
            tree,
            symbols,
            types: self.types,
        }
    }

    /// Reborrow this context for one explicit module-local view and one explicit type table.
    pub(crate) fn reborrow_for_module_with_options_and_types<'b>(
        &'b self,
        module: &'b Module,
        options: &'b AnalyzeOptions,
        tree: &'b NodeTree,
        symbols: &'b SymbolTable,
        types: &'b mut TypeTable,
    ) -> TypeTablesContext<'b> {
        TypeTablesContext {
            module,
            profile: self.profile,
            options,
            tree,
            symbols,
            types,
        }
    }
}

/// Shared mutable infer context for tree, symbols, types, and infer tables.
#[derive(Debug)]
pub(crate) struct InferTablesContext<'a> {
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

impl<'a> InferTablesContext<'a> {
    /// Construct an infer context for table-driven operations.
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
    pub(crate) fn reborrow(&mut self) -> InferTablesContext<'_> {
        InferTablesContext {
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
    pub(crate) fn type_tables_reborrow(&mut self) -> TypeTablesContext<'_> {
        TypeTablesContext {
            module: self.module,
            profile: self.profile,
            options: self.options,
            tree: self.tree,
            symbols: self.symbols,
            types: self.types,
        }
    }

    /// Reborrow this context as a type-resolution context for one module-local view with explicit options and one explicit type table.
    pub(crate) fn type_tables_reborrow_for_module_with_options_and_types<'b>(
        &'b self,
        module: &'b Module,
        options: &'b AnalyzeOptions,
        tree: &'b NodeTree,
        symbols: &'b SymbolTable,
        types: &'b mut TypeTable,
    ) -> TypeTablesContext<'b> {
        TypeTablesContext {
            module,
            profile: self.profile,
            options,
            tree,
            symbols,
            types,
        }
    }

    /// Split this infer context into type tables and infer table borrows.
    pub(crate) fn split_type_tables_and_infer(
        &mut self,
    ) -> (TypeTablesContext<'_>, &mut InferTable) {
        (
            TypeTablesContext {
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
