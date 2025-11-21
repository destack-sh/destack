use dyst_ast as ast;
use dyst_dir::{LocalNodeId, LocalScopeId, Module, NodeTree, WithClause};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Bind a with clause to a DIR with clause.
    pub(super) fn bind_with_clause(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        with_clause_id: ast::LocalNodeId<ast::WithClause>,
        tree: &mut NodeTree,
    ) -> LocalNodeId<WithClause> {
        let with_clause = module.get(with_clause_id);
        let alias = with_clause
            .alias
            .map(|alias| self.session.strings.intern_from(&module.ast_strings, alias));
        let right = self.bind_expression(module, scope_id, with_clause.right, tree);
        tree.insert_from_source(WithClause { alias, right }, module.id, with_clause_id)
    }
}
