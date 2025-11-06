use dyst_ast as ast;
use dyst_dir::{NodeId, WithClause};
use dyst_source::FileId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a with clause to a DIR with clause.
    pub fn lower_with_clause(
        &mut self,
        source_id: FileId,
        ast: &ast::NodeTree,
        with_clause_id: ast::NodeId<ast::WithClause>,
    ) -> NodeId<WithClause> {
        let with_clause = ast.get(with_clause_id);
        let alias = with_clause
            .alias
            .map(|alias| self.intern_string(source_id, alias));
        let right = self.lower_expression(source_id, ast, with_clause.right);
        self.tree
            .insert_from_ast(WithClause { alias, right }, source_id, with_clause_id)
    }
}
