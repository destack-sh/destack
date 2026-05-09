use crate::Compiler;
use crate::common::ast::evaluate_numeric_literal;
use destack_artifact::Ast;
use destack_ast::{self as ast, StringId};
use destack_dir::{
    BindingTable, DeclaredModule, Key, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Name,
    SymbolSpace, Tree, TypeTable,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind an AST name into a DIR name.
    pub(super) fn bind_name(&self, _ast: &Ast, name: ast::Name) -> Name {
        // intern the name into the program string pool
        let name_id = name.string();
        let name_id = name_id;

        // preserve the name flavor
        match name {
            ast::Name::Identifier(_) => Name::Identifier(name_id),
            ast::Name::String(_) => Name::String(name_id),
            ast::Name::Number(_) => Name::Number(name_id),
        }
    }

    /// Bind a key to a DIR key.
    pub(super) fn bind_key(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        key: ast::Key,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
    ) -> Key {
        match key {
            ast::Key::Name(ast::Name::Number(string_id)) => {
                // numeric keys evaluate to canonical string representation
                let source = self.repository.string_pool().get(string_id);
                let canonical = evaluate_numeric_literal(&source);
                let name = StringId::for_text(&canonical);
                Key::Name(Name::Number(name))
            }
            ast::Key::Name(name) => {
                let name = self.bind_name(ast, name);
                Key::Name(name)
            }
            ast::Key::Private(name) => {
                let name = name;
                Key::Private(name)
            }
            ast::Key::Expression(expression) => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_scope,
                    declared_modules,
                    scope,
                    expression,
                    parent_id,
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                Key::Expression(expression)
            }
        }
    }
}
