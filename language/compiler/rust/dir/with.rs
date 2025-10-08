use dyst_ast as ast;
use dyst_dir::{NodeId, WithClause};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a with clause to a DIR with clause.
    pub fn lower_with_clause(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        with_clause: ast::NodeId<ast::WithClause>,
    ) -> NodeId<WithClause> {
        let with_clause = ast.get(with_clause);
        self.tree.allocate(WithClause {
            alias: with_clause
                .alias
                .map(|alias| self.intern_string(source_id, alias)),
            right: self.lower_expression(source_id, ast, with_clause.right),
        })
    }
}
