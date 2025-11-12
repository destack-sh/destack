use dyst_ast as ast;
use dyst_dir::{Module, NodeId, WhereClause};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a where clause to a DIR where clause.
    pub fn lower_where_clause(
        &mut self,
        module: &Module,
        where_clause_id: ast::NodeId<ast::WhereClause>,
    ) -> NodeId<WhereClause> {
        let where_clause = module.get(where_clause_id);
        match where_clause {
            ast::WhereClause::Assertion { left, right } => {
                let left = self.session.strings.intern_from(&module.strings, *left);
                let right = self.lower_expression(module, *right);
                self.session.tree.insert_from_ast(
                    WhereClause::Assertion { left, right },
                    module.id,
                    where_clause_id,
                )
            }
            ast::WhereClause::Guard { guard } => {
                let guard = self.lower_expression(module, *guard);
                self.session.tree.insert_from_ast(
                    WhereClause::Guard { guard },
                    module.id,
                    where_clause_id,
                )
            }
        }
    }
}
