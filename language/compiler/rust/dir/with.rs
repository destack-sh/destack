use dyst_ast as ast;
use dyst_dir::{NodeId, WithClause};

use crate::{AstNodeId, Compiler};

impl<'a> Compiler<'a> {
    /// Lower a with clause to a DIR with clause.
    pub fn lower_with_clause(
        &mut self,
		ast: &ast::NodeTree,
        with_clause: AstNodeId<ast::WithClause>,
    ) -> Option<NodeId<WithClause>> {
        todo!("Compiler.lower_with_clause")
    }
}
