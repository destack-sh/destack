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
        symbols: &'a SymbolTable,
        types: &'a mut TypeTable,
        options: &'a AnalyzeOptions,
    ) -> Self {
        Self {
            module,
            profile,
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
            symbols: self.symbols,
            types: self.types,
            options: self.options,
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
}
