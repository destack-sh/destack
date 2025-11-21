use dyst_ast as ast;
use dyst_dir::{LocalNodeId, LocalScopeId, Module, NodeTree, WhereClause};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Bind a where clause to a DIR where clause.
    pub(super) fn bind_where_clause(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        where_clause_id: ast::LocalNodeId<ast::WhereClause>,
        tree: &mut NodeTree,
    ) -> LocalNodeId<WhereClause> {
        let where_clause = module.get(where_clause_id);
        match where_clause {
            ast::WhereClause::Assertion { left, right } => {
                let left = self.session.strings.intern_from(&module.ast_strings, *left);
                let right = self.bind_expression(module, scope_id, *right, tree);
                tree.insert_from_source(
                    WhereClause::Assertion { left, right },
                    module.id,
                    where_clause_id,
                )
            }
            ast::WhereClause::Guard { guard } => {
                let guard = self.bind_expression(module, scope_id, *guard, tree);
                tree.insert_from_source(WhereClause::Guard { guard }, module.id, where_clause_id)
            }
        }
    }
}
