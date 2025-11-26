use dyst_ast as ast;
use dyst_dir::{LocalNodeId, LocalScopeId, LocalScopeMark, Module, NodeTree, SymbolTable, TypeTable, WithClause};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a with clause to a DIR with clause.
    pub(super) fn bind_with_clause(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        with_clause_id: ast::LocalNodeId<ast::WithClause>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<WithClause> {
        let with_clause = module.get(with_clause_id);
        let alias = with_clause
            .alias
            .map(|alias| self.program.strings.intern_from(&module.ast_strings, alias));
        let right = self.bind_expression(module, scope, with_clause.right, tree, symbols, types);
        tree.insert_from_source(WithClause { alias, right }, with_clause_id, scope)
    }
}
