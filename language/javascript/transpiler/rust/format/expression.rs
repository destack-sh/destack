use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Expression, NodeId};

use crate::format::literal::format_scalar_literal;
use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: NodeId<Expression>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Expression::ScalarLiteral { value } => {
                format_scalar_literal(value, f)?;
            }
            _ => todo!("format_node{self:?}"),
        }

        Ok(())
    }
}
