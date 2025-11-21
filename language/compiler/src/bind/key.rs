use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Key, LocalScopeId, Module, NodeTree, SymbolTable};

impl<'a> Compiler<'a> {
    /// Bind a key to a DIR key.
    pub(super) fn bind_key(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        key: ast::Key,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> Key {
        match key {
            ast::Key::Name(name) => {
                let name = self
                    .session
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                Key::Name(name)
            }
            ast::Key::Expression(expression) => {
                let expression = self.bind_expression(module, scope_id, expression, tree, symbols);
                Key::Expression(expression)
            }
            ast::Key::NamedExpression { name, key } => {
                let name = self.session.strings.intern_from(&module.ast_strings, name);
                let key = self.bind_expression(module, scope_id, key, tree, symbols);
                Key::NamedExpression { name, key }
            }
        }
    }
}
