use dyst_language_fir::format::FormatResult;
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

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

            Expression::Let(node) => node.format(f),
            Expression::Block(node) => node.format(f),
            Expression::If(node) => node.format(f),
            Expression::While(node) => node.format(f),
            Expression::For(node) => node.format(f),
            Expression::Loop(node) => node.format(f),
            Expression::Break(node) => node.format(f),
            Expression::Continue(node) => node.format(f),
            Expression::Defer(node) => node.format(f),
            Expression::Return(node) => node.format(f),
            Expression::Try(node) => node.format(f),
            Expression::Match(node) => node.format(f),

            Expression::Path(p) => p.format(f),
            Expression::ScalarLiteral(node) => node.format(f),
            Expression::RangeLiteral(node) => node.format(f),
            Expression::ArrayLiteral(node) => node.format(f),
            Expression::TupleLiteral(node) => node.format(f),
            Expression::StructLiteral(node) => node.format(f),

            Expression::Unary { operator, right } => todo!("unary {operator:?} {right:?}"),
            Expression::Reference { mutability, right } => {
                todo!("reference {mutability:?} {right:?}")
            }
            Expression::Member { receiver, path } => todo!("member {receiver:?}.{path:?}"),
            Expression::Index(node) => todo!("index {node:?}"),
            Expression::Call(node) => node.format(f),
            Expression::Cast(node) => todo!("cast {node:?}"),
            Expression::Unwrap(expr) => write!(f, [expr, token("?")]),
            Expression::UnwrapOrPanic(expr) => write!(f, [expr, token("!")]),
            Expression::Coalesce(node) => todo!("coalesce {node:?}"),
            Expression::Binary {
                left,
                operator,
                right,
            } => todo!("binary {left:?} {operator:?} {right:?}"),
            Expression::Assign {
                left,
                operator,
                right,
            } => todo!("assign {left:?} {operator:?} {right:?}"),

            Expression::Error => todo!("error"),
        }
    }
}
