use crate::{Asynchrony, Expression, FunctionCardinality, Keyword, LocalNodeId, PostfixPosition};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::argument::list_like;
use crate::format::literal::{format_scalar_literal, format_template_literal};
use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Expression>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Expression::Declaration { declaration } => {
                write!(f, [declaration])?;
            }
            Expression::ArrowFunction { signature, body } => {
                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // cardinality
                if signature.cardinality == FunctionCardinality::Generator {
                    write!(f, [token("*")])?;
                }

                // static parameters
                if f.context().include_types()
                    && let Some(static_parameters) = signature
                        .generics
                        .as_ref()
                        .and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // dynamic parameters
                write!(f, [list_like("(", ")", ",", &signature.dynamic_parameters)])?;

                // return type
                if f.context().include_types()
                    && let Some(return_type) = signature.return_type
                {
                    write!(f, [token(":"), space(), return_type])?;
                }

                // body
                write!(f, [space(), token("=>"), space(), body])?;
            }
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
            Expression::ObjectLiteral { properties } => {
                write!(f, [list_like("{", "}", ",", properties).include_space()])?;
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
            Expression::Assign { left, right } => {
                write!(f, [left, space(), token("="), space(), right])?;
            }
            Expression::AssignBinary {
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
                name,
                static_arguments,
            } => {
                write!(f, [left, token("."), *name])?;
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
                static_arguments,
                dynamic_arguments,
            } => {
                write!(f, [left])?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                if f.context().include_types()
                    && let Some(static_arguments) = static_arguments
                {
                    write!(f, [list_like("<", ">", ",", static_arguments)])?;
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

            Expression::Stub => {}

            Expression::Error => {
                write!(f, [token("/* ERROR */")])?;
            }
        }

        Ok(())
    }
}
