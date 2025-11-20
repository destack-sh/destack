use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Key, Module, ScopeId};

impl<'a> Compiler<'a> {
    /// Lower a key to a DIR key.
    pub(super) fn lower_key(&self, module: &Module, scope_id: ScopeId, key: ast::Key) -> Key {
        match key {
            ast::Key::Name(name) => {
                let name = self
                    .session
                    .strings
                    .intern_from(&module.strings, name.string());
                Key::Name(name)
            }
            ast::Key::Expression(expression) => {
                let expression = self.lower_expression(module, scope_id, expression);
                Key::Expression(expression)
            }
            ast::Key::NamedExpression { name, key } => {
                let name = self.session.strings.intern_from(&module.strings, name);
                let key = self.lower_expression(module, scope_id, key);
                Key::NamedExpression { name, key }
            }
        }
    }
}
