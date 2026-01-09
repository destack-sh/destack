use destack_dir::{Expression, LocalNodeId, LocalScopeMark, SymbolSpaceOrder};

use destack_workspace::{Module, ModuleAst};

use crate::Compiler;

impl Compiler {
    /// Bind the AST root expressions for a module.
    pub(super) fn bind_module_roots(
        &self,
        module: &Module,
        ast: &ModuleAst,
    ) -> Vec<LocalNodeId<Expression>> {
        let dir = module.dir_base();
        let mut tree = dir.tree.write();
        let mut symbols = dir.symbols.write();
        let mut types = dir.types.write();
        let scope = (dir.namespace_scope, LocalScopeMark::end());
        ast.roots
            .iter()
            .map(|expression| {
                self.bind_expression(
                    module,
                    ast,
                    scope,
                    *expression,
                    None,
                    &mut tree,
                    &mut symbols,
                    &mut types,
                    SymbolSpaceOrder::ValueThenType,
                )
            })
            .collect()
    }
}
