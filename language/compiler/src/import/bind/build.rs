use destack_dir::{
    BindingTable, DeclaredModule, Expression, LocalNodeId, LocalScopeId, LocalScopeMark,
    SymbolSpace, Tree, TypeTable,
};

use destack_artifact::Ast;
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Bind the AST root expressions for a module.
    pub(super) fn bind_module_roots(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
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
                    global_scope,
                    declared_modules,
                    scope,
                    *expression,
                    None,
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                )
            })
            .collect()
    }
}
