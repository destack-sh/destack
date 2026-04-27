use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR name to an AST name.
    pub(super) fn unbind_name(&self, ast_strings: &mut StringPool, name: dir::Name) -> ast::Name {
        // intern the name into the ast string pool
        let name_id = ast_strings.intern_from(&self.repository.strings, name.string());

        // preserve the name flavor
        match name {
            dir::Name::Identifier(_) => ast::Name::Identifier(name_id),
            dir::Name::String(_) => ast::Name::String(name_id),
            dir::Name::Number(_) => ast::Name::Number(name_id),
        }
    }

    /// Unbind a DIR dynamic key to an AST key.
    pub(super) fn unbind_key(
        &self,
        module: &Module,
        key: &dir::Key,
        tree: &dir::Tree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::Key {
        match key {
            dir::Key::Name(name) => {
                let name = self.unbind_name(ast_strings, *name);
                ast::Key::Name(name)
            }
            dir::Key::Private(name) => {
                let name = ast_strings.intern_from(&self.repository.strings, *name);
                ast::Key::Private(name)
            }
            dir::Key::Expression(expression) => {
                let expression = self.unbind_expression(
                    module,
                    *expression,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Key::Expression(expression)
            }
        }
    }
}
