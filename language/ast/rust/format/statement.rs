use crate::{DystFormatter, FormatNode, NodeId, Statement};
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, Statement> for Statement {
    fn format_node(
        &self,
        node_id: NodeId<Statement>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        match self {
            Statement::With(node) => node.format(f)?,
            Statement::Use(node) => node.format(f)?,
            Statement::Break(node) => node.format(f)?,
            Statement::Continue(node) => node.format(f)?,
            Statement::Defer(node) => node.format(f)?,
            Statement::Return(node) => node.format(f)?,

            Statement::Expression(node) => node.format(f)?,
        }
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        Ok(())
    }
}
