use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Key, LocalScopeId, Module, NodeTree, SymbolTable, TypeTable};

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Bind a key to a DIR key.
    pub(super) fn bind_key(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        key: ast::Key,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Key {
        match key {
            ast::Key::Name(name) => {
                let name = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                Key::Name(name)
            }
            ast::Key::Expression(expression) => {
                let expression =
                    self.bind_expression(module, scope_id, expression, tree, symbols, types);
                Key::Expression(expression)
            }
            ast::Key::NamedExpression { name, key } => {
                let name = self.program.strings.intern_from(&module.ast_strings, name);
                let key = self.bind_expression(module, scope_id, key, tree, symbols, types);
                Key::NamedExpression { name, key }
            }
        }
    }
}
