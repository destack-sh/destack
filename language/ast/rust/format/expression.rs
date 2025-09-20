use dyst_language_fir::format::FormatResult;
use dyst_language_fir::prelude::*;

use crate::{DystFormatter, Expression, FormatNode, NodeId};

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        _node_id: NodeId<Expression>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Expression::Module(node) => node.format(f),
            Expression::Struct(node) => node.format(f),
            Expression::Enum(node) => node.format(f),
            Expression::Union(node) => node.format(f),
            Expression::Trait(node) => node.format(f),
            Expression::Implement(node) => node.format(f),
            Expression::Function(node) => node.format(f),

            Expression::Path(p) => p.format(f),
            Expression::ScalarLiteral(node) => node.format(f),
            Expression::RangeLiteral(node) => node.format(f),
            Expression::TupleLiteral(node) => node.format(f),

            _ => todo!(),
        }
    }
}
