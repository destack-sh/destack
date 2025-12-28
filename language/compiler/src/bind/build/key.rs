use super::literal::evaluate_numeric_literal;
use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    DynamicKey, LocalNodeIdAny, LocalScopeId, LocalScopeMark, NodeTree, SymbolTable, TypeTable,
};
use destack_workspace::{Module, ModuleAst};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a key to a DIR key.
    pub(super) fn bind_key(
        &self,
        module: &Module,
        ast: &ModuleAst,
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
            ast::Key::Expression(expression) => {
                let expression = self.bind_expression(
                    module, ast, scope, expression, parent_id, tree, symbols, types,
                );
                DynamicKey::Expression(expression)
            }
            ast::Key::NamedExpression { name, key } => {
                let name = self.program.strings.intern_from(&ast.strings, name);
                let key =
                    self.bind_expression(module, ast, scope, key, parent_id, tree, symbols, types);
                DynamicKey::NamedExpression { name, key }
            }
        }
    }
}
