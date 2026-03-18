use destack_dir::{
    Expression, LocalNodeId, LocalScopeId, LocalScopeMark, ModuleBinding, NodeTree,
    SymbolSpaceOrder, SymbolTable, TypeTable,
};

use destack_workspace::{Ast, Module};

use crate::Compiler;

impl Compiler {
    /// Bind the AST root expressions for a module.
    pub(super) fn bind_module_roots(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Vec<LocalNodeId<Expression>> {
        let scope = (namespace_scope, LocalScopeMark::end());
        ast.roots
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
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
