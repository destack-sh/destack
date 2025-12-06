use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    DynamicKey, LocalNodeIdAny, LocalScopeId, LocalScopeMark, NodeTree, SymbolTable, TypeTable,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a key to a DIR key.
    pub(super) fn bind_key(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        key: ast::Key,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> DynamicKey {
        match key {
            ast::Key::Name(name) => {
                let name = self
                    .program
                    .strings
                    .intern_from(&module.ast.strings, name.string());
                DynamicKey::Name(name)
            }
            ast::Key::Expression(expression) => {
                let expression = self
                    .bind_expression(module, scope, expression, parent_id, tree, symbols, types);
                DynamicKey::Expression(expression)
            }
            ast::Key::NamedExpression { name, key } => {
                let name = self.program.strings.intern_from(&module.ast.strings, name);
                let key = self.bind_expression(module, scope, key, parent_id, tree, symbols, types);
                DynamicKey::NamedExpression { name, key }
            }
        }
    }
}
