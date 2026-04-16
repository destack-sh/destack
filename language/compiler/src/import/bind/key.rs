use crate::Compiler;
use crate::analyze::evaluate_numeric_literal;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    Key, LocalNodeIdAny, LocalScopeId, LocalScopeMark, ModuleBinding, Name, NodeTree,
    SymbolSpaceOrder, SymbolTable, TypeTable,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind an AST name into a DIR name.
    pub(super) fn bind_name(&self, ast: &Ast, name: ast::Name) -> Name {
        // intern the name into the program string pool
        let name_id = name.string();
        let name_id = self.repository.strings.intern_from(&ast.strings, name_id);

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
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        key: ast::Key,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> Key {
        match key {
            ast::Key::Name(ast::Name::Number(string_id)) => {
                // numeric keys evaluate to canonical string representation
                let source = ast.strings.get(string_id);
                let canonical = evaluate_numeric_literal(&source);
                let name = self.repository.strings.intern(&canonical);
                Key::Name(Name::Number(name))
            }
            ast::Key::Name(name) => {
                let name = self.bind_name(ast, name);
                Key::Name(name)
            }
            ast::Key::Private(name) => {
                let name = self.repository.strings.intern_from(&ast.strings, name);
                Key::Private(name)
            }
            ast::Key::Expression(expression) => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    expression,
                    parent_id,
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueOnly,
                );
                Key::Expression(expression)
            }
        }
    }
}
