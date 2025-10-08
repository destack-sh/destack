use dyst_ast as ast;
use dyst_dir::{Expression, NodeId};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower an expression to a DIR expression.
    pub fn lower_expression(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        expression: ast::NodeId<ast::Expression>,
    ) -> NodeId<Expression> {
		todo!("Compiler::lower_expression");
    }
}
