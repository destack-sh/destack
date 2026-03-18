use crate::Compiler;
use crate::analyze::evaluate_numeric_literal;
use destack_ast as ast;
use destack_dir::{
    DynamicKey, LocalNodeIdAny, LocalScopeId, LocalScopeMark, ModuleBinding, Name, NodeTree,
    SymbolSpaceOrder, SymbolTable, TypeTable,
};
use destack_workspace::{Ast, Module};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind an AST name into a DIR name.
    pub(super) fn bind_name(&self, ast: &Ast, name: ast::Name) -> Name {
        // intern the name into the program string pool
        let name_id = name.string();
        let name_id = self.program.strings.intern_from(&ast.strings, name_id);

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
    ) -> DynamicKey {
        match key {
            ast::Key::Name(ast::Name::Number(string_id)) => {
                // numeric keys evaluate to canonical string representation
                let source = ast.strings.get(string_id);
                let canonical = evaluate_numeric_literal(&source);
                let name = self.program.strings.intern(&canonical);
                DynamicKey::Number(name)
            }
            ast::Key::Name(name) => {
                let name = self
                    .program
                    .strings
                    .intern_from(&ast.strings, name.string());
                DynamicKey::Name(name)
            }
            ast::Key::Private(name) => {
                let name = self.program.strings.intern_from(&ast.strings, name);
                DynamicKey::Private(name)
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
                DynamicKey::Expression(expression)
            }
            ast::Key::NamedExpression { name, key } => {
                let name = self.program.strings.intern_from(&ast.strings, name);
                let key = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    key,
                    parent_id,
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                );
                DynamicKey::NamedExpression { name, key }
            }
        }
    }
}
