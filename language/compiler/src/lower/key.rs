use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Key, Module};

impl<'a> Compiler<'a> {
    /// Lower a key to a DIR key.
    pub fn lower_key(&mut self, module: &Module, key: ast::Key) -> Key {
        match key {
            ast::Key::Name(name) => {
                let name = self
                    .session
                    .strings
                    .intern_from(&module.strings, name.string());
                Key::Name(name)
            }
            ast::Key::Expression(expression) => {
                let expression = self.lower_expression(module, expression);
                Key::Expression(expression)
            }
            ast::Key::NamedExpression { name, key } => {
                let name = self.session.strings.intern_from(&module.strings, name);
                let key = self.lower_expression(module, key);
                Key::NamedExpression { name, key }
            }
        }
    }
}
