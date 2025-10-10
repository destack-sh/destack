use dyst_ast as ast;
use dyst_dir::{NodeId, UseItem};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a use clause to a DIR use items.
    pub fn lower_use_clause(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        use_clause_id: ast::NodeId<ast::UseClause>,
    ) -> Vec<NodeId<UseItem>> {
        todo!("Compiler::lower_use_clause");
    }
}
