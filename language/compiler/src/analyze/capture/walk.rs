use indexmap::IndexMap;

use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalScopeId, NodeTree, NodeVisitor,
    NodeVisitorOptions, SymbolTable, walk_expression,
};
use destack_source::ModuleId;
use destack_workspace::Module;

use crate::Compiler;

/// Collect captures for a closure body.
pub(super) struct CaptureCollector<'a> {
    /// The module id for capture checks.
    pub module_id: ModuleId,
    /// The scope id for the closure body.
    pub closure_scope: LocalScopeId,
    /// The node tree for this module.
    pub tree: &'a NodeTree,
    /// The symbol table for this module.
    pub symbols: &'a SymbolTable,
    /// The compiler context for helper access.
    pub compiler: &'a Compiler,
    /// The module for this analysis pass.
    pub module: &'a Module,
    /// The captured symbols in discovery order.
    pub captured_symbols: IndexMap<GlobalSymbolId, ()>,
    /// The captured symbol bound to `this`.
    pub this_symbol: Option<GlobalSymbolId>,
}

impl<'a> CaptureCollector<'a> {
    /// Build a collector for a single closure body.
    pub(super) fn new(
        module_id: ModuleId,
        closure_scope: LocalScopeId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        compiler: &'a Compiler,
        module: &'a Module,
    ) -> Self {
        // initialize collector state
        Self {
            module_id,
            closure_scope,
            tree,
            symbols,
            compiler,
            module,
            captured_symbols: IndexMap::new(),
            this_symbol: None,
        }
    }

    /// Collect captures starting from an expression.
    pub(super) fn collect(&mut self, expression_id: LocalNodeId<Expression>) {
        self.visit_expression(expression_id);
    }

    /// Visit an expression node for captures.
    fn visit_expression(&mut self, expression_id: LocalNodeId<Expression>) {
        // skip nested declarations
        let expression = self.tree.get(expression_id);
        if matches!(expression, Expression::Declaration { .. }) {
            return;
        }

        // capture value references and this
        match expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                self.capture_symbol(*target_symbol);
            }
            Expression::This => {
                if let Some(symbol) = self.compiler.resolve_this_symbol(
                    self.module,
                    expression_id,
                    self.tree,
                    self.symbols,
                ) && self.capture_symbol(symbol)
                {
                    self.this_symbol = Some(symbol);
                }
            }
            _ => {}
        }

        // walk child nodes
        walk_expression(self, self.tree, expression_id, expression);
    }

    /// Record a captured symbol if it should be captured.
    fn capture_symbol(&mut self, symbol: GlobalSymbolId) -> bool {
        // skip symbols that do not require capture
        if !self.compiler.should_capture_symbol(
            self.module_id,
            self.symbols,
            self.closure_scope,
            symbol,
        ) {
            return false;
        }

        // preserve capture ordering
        self.captured_symbols.entry(symbol).or_insert(());
        true
    }
}

impl<'a> NodeVisitor for CaptureCollector<'a> {
    /// Return the visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        static OPTIONS: NodeVisitorOptions = NodeVisitorOptions {};
        &OPTIONS
    }

    /// Visit an expression node.
    fn visit_expression(
        &mut self,
        _tree: &NodeTree,
        id: LocalNodeId<Expression>,
        _expression: &Expression,
    ) {
        self.visit_expression(id);
    }
}
