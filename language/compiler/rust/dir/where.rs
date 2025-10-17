use dyst_ast as ast;
use dyst_dir::{NodeId, WhereClause};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a where clause to a DIR where clause.
    pub fn lower_where_clause(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        where_clause_id: ast::NodeId<ast::WhereClause>,
    ) -> NodeId<WhereClause> {
        let where_clause = ast.get(where_clause_id);
        match where_clause {
            ast::WhereClause::Assertion { left, right } => {
                let left = self.intern_string(source_id, *left);
                let right = self.lower_expression(source_id, ast, *right);
                self.tree.insert_from_ast(
                    WhereClause::Assertion { left, right },
                    source_id,
                    where_clause_id,
                )
            }
            ast::WhereClause::Guard { guard } => {
                let guard = self.lower_expression(source_id, ast, *guard);
                self.tree
                    .insert_from_ast(WhereClause::Guard { guard }, source_id, where_clause_id)
            }
        }
    }
}
