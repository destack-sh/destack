use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR dynamic key to an AST key.
    pub(super) fn unbind_key(
        &self,
        module: &Module,
        key: &dir::DynamicKey,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
    ) -> ast::Key {
        match key {
            dir::DynamicKey::Name(name) => {
                let name = ast_strings.intern_from(&self.program.strings, *name);
                ast::Key::Name(ast::Name::Identifier(name))
            }
            dir::DynamicKey::Expression(expression) => {
                let expression = self.unbind_expression(
                    module,
                    *expression,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                );
                ast::Key::Expression(expression)
            }
            dir::DynamicKey::NamedExpression { name, key } => {
                let name = ast_strings.intern_from(&self.program.strings, *name);
                let key =
                    self.unbind_expression(module, *key, tree, symbols, ast_tree, ast_strings);
                ast::Key::NamedExpression { name, key }
            }
        }
    }
}
