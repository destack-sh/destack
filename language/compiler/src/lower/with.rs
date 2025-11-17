use dyst_ast as ast;
use dyst_dir::{Module, NodeId, WithClause};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a with clause to a DIR with clause.
    pub(super) fn lower_with_clause(
        &mut self,
        module: &Module,
        with_clause_id: ast::NodeId<ast::WithClause>,
    ) -> NodeId<WithClause> {
        let with_clause = module.get(with_clause_id);
        let alias = with_clause
            .alias
            .map(|alias| self.session.strings.intern_from(&module.strings, alias));
        let right = self.lower_expression(module, with_clause.right);
        self.session
            .tree
            .insert_from_ast(WithClause { alias, right }, module.id, with_clause_id)
    }
}
