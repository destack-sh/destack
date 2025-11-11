use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::{Expression, NodeId, PostfixPosition};

use crate::format::argument::list_like;
use crate::format::literal::{format_scalar_literal, format_template_literal};
use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        _node_id: NodeId<Expression>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Expression::Path {
                path,
                static_arguments,
            } => {
                write!(f, [path])?;
                if f.context().include_types()
                    && let Some(static_arguments) = static_arguments
                {
                    write!(f, [list_like("<", ">", ",", static_arguments)])?;
                }
            }
            Expression::ScalarLiteral { value } => {
                format_scalar_literal(value, f)?;
            }
            Expression::TemplateLiteral { value } => {
                format_template_literal(value, f)?;
            }
            Expression::ArrayLiteral { elements } => {
                write!(f, [list_like("[", "]", ",", elements)])?;
            }
            Expression::ObjectLiteral { fields } => {
                write!(f, [list_like("{", "}", ",", fields).include_space()])?;
            }

            Expression::Parenthesized { expression } => {
                write!(f, [token("("), expression, token(")")])?;
            }

            Expression::TypeUnary { operator, right } => {
                if operator.is_prefix() {
                    write!(f, [operator, right])?;
                } else {
                    write!(f, [right, operator])?;
                }
            }
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                write!(f, [left, space(), operator, space(), right])?;
            }
            Expression::Unary { operator, right } => {
                if operator.is_prefix() {
                    write!(f, [operator, right])?;
                } else {
                    write!(f, [right, operator])?;
                }
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                write!(f, [left, space(), operator, space(), right])?;
            }

            Expression::Maybe { position, left } => {
                write!(f, [left])?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                write!(f, [token("?")])?;
            }
            Expression::Must { position, left } => {
                write!(f, [left])?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                write!(f, [token("!")])?;
            }
            Expression::Member {
                left,
                path,
                static_arguments,
            } => {
                write!(f, [left, token("."), path])?;
                if f.context().include_types()
                    && let Some(static_arguments) = static_arguments
                {
                    write!(f, [list_like("<", ">", ",", static_arguments)])?;
                }
            }
            Expression::Index {
                position,
                left,
                right,
            } => {
                write!(f, [left])?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                write!(f, [token("["), right, token("]")])?;
            }
            Expression::Call {
                position,
                left,
                dynamic_arguments,
            } => {
                write!(f, [left])?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                write!(f, [list_like("(", ")", ",", dynamic_arguments)])?;
            }
            Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                write!(f, [token("new"), space(), left])?;
                if f.context().include_types()
                    && let Some(static_arguments) = static_arguments
                {
                    write!(f, [list_like("<", ">", ",", static_arguments)])?;
                }
                write!(f, [list_like("(", ")", ",", dynamic_arguments)])?;
            }

            Expression::IfTernary {
                condition,
                then_expression,
                else_expression,
            } => {
                write!(
                    f,
                    [
                        condition,
                        space(),
                        token("?"),
                        space(),
                        then_expression,
                        space(),
                        token(":"),
                        space(),
                        else_expression
                    ]
                )?;
            }

            _ => todo!("format_node{self:?}"),
        }

        Ok(())
    }
}
