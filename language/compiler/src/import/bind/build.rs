use destack_dir::{
    Expression, LocalNodeId, LocalScopeMark, NodeTree, SymbolSpaceOrder, SymbolTable, TypeTable,
};

use destack_workspace::{ImportDir, Module, ModuleAst};

use crate::Compiler;

impl Compiler {
    /// Bind the AST root expressions for a module.
    pub(super) fn bind_module_roots(
        &self,
        module: &Module,
        ast: &ModuleAst,
        dir: &mut ImportDir,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Vec<LocalNodeId<Expression>> {
        let scope = (dir.namespace_scope, LocalScopeMark::end());
        ast.roots
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    ast,
                    dir,
                    scope,
                    *expression,
                    None,
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueThenType,
                )
            })
            .collect()
    }
}
