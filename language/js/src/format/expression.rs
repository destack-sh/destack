use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::function::format_function_parameters;
use crate::format::list::delimited;
use crate::format::literal::{format_scalar_literal, format_template_literal};
use crate::{
    ArrayElement, ArrowFunctionBody, Asynchrony, BinaryOperator, Expression, FormatNode, Formatter,
    Keyword, LocalNodeId, Precedence, UnaryOperator, UpdatePosition, format_attributed,
};

impl<'ast> FormatNode<'ast> for Expression {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        format_expression_with_precedence(self, Precedence::Lowest, f)
    }
}

/// Format one expression in statement position.
pub(crate) fn format_expression_statement<'ast>(
    id: LocalNodeId<Expression>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(id);
    let needs_parentheses = expression.needs_statement_parentheses(f.context().tree);

    if needs_parentheses {
        write!(f, [token("(")])?;
    }
    format_expression_id_with_precedence(id, Precedence::Lowest, f)?;
    if needs_parentheses {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one expression in an ECMAScript `NoIn` context.
pub(crate) fn format_expression_without_in<'ast>(
    id: LocalNodeId<Expression>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let needs_parentheses = f
        .context()
        .tree
        .get(id)
        .needs_no_in_parentheses(f.context().tree);

    if needs_parentheses {
        write!(f, [token("(")])?;
    }
    format_expression_id_with_precedence(id, Precedence::Lowest, f)?;
    if needs_parentheses {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one expression with one required parent precedence.
fn format_expression_with_precedence<'ast>(
    expression: &Expression,
    parent_precedence: Precedence,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let current_precedence = expression.precedence();
    let needs_wrap = current_precedence < parent_precedence;

    // outer wrap
    if needs_wrap {
        write!(f, [token("(")])?;
    }

    match expression {
        Expression::Declaration { declaration } => {
            write!(f, [declaration])?;
        }
        Expression::ArrowFunction {
            asynchrony,
            parameters,
            rest,
            body,
        } => {
            // asynchrony
            if *asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Async, space()])?;
            }

            // parameters
            format_function_parameters(parameters, *rest, f)?;

            // body
            write!(f, [space(), token("=>"), space()])?;
            match body {
                ArrowFunctionBody::Expression(body) => {
                    let body_expression = f.context().tree.get(*body);
                    let needs_parentheses =
                        body_expression.needs_arrow_parentheses(f.context().tree);
                    if needs_parentheses {
                        write!(f, [token("(")])?;
                    }
                    format_expression_id_with_precedence(*body, Precedence::Assignment, f)?;
                    if needs_parentheses {
                        write!(f, [token(")")])?;
                    }
                }
                ArrowFunctionBody::Block(body) => {
                    write!(f, [body])?;
                }
            }
        }
        Expression::Identifier { identifier } => {
            write!(f, [identifier])?;
        }
        Expression::ImportMeta => {
            write!(f, [token("import"), token("."), token("meta")])?;
        }
        Expression::This => {
            write!(f, [Keyword::This])?;
        }
        Expression::Super => {
            write!(f, [Keyword::Super])?;
        }
        Expression::Literal { value } => {
            format_scalar_literal(value, f)?;
        }
        Expression::TemplateLiteral { value } => {
            format_template_literal(value, f)?;
        }
        Expression::ArrayLiteral { elements } => {
            let ends_in_elision = elements.last().is_some_and(|element| {
                matches!(f.context().tree.get(*element), ArrayElement::Elision)
            });
            let mut elements = delimited("[", "]", ",", elements);
            if ends_in_elision {
                elements.with_trailing_separator();
            }
            write!(f, [elements])?;
        }
        Expression::SequenceExpression { expressions } => {
            for (index, expression_id) in expressions.iter().enumerate() {
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }

                format_expression_id_with_precedence(*expression_id, Precedence::Assignment, f)?;
            }
        }
        Expression::ObjectLiteral { properties } => {
            let mut properties = delimited("{", "}", ",", properties);
            properties.include_space();
            write!(f, [properties])?;
        }
        Expression::Parenthesized { expression } => {
            write!(f, [token("(")])?;
            format_expression_id_with_precedence(*expression, Precedence::Lowest, f)?;
            write!(f, [token(")")])?;
        }
        Expression::Await { value } => {
            write!(f, [Keyword::Await, space()])?;
            format_expression_id_with_precedence(*value, Precedence::Prefix, f)?;
        }
        Expression::Yield { is_delegate, value } => {
            write!(f, [Keyword::Yield])?;

            if *is_delegate {
                write!(f, [token("*"), space()])?;
            } else if value.is_some() {
                write!(f, [space()])?;
            }

            if let Some(value) = value {
                format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
            }
        }
        Expression::Unary { operator, right } => {
            let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);
            if needs_space {
                write!(f, [operator, space()])?;
            } else {
                write!(f, [operator])?;
            }
            format_expression_id_with_precedence(*right, Precedence::Prefix, f)?;
        }
        Expression::Update {
            place,
            operator,
            position,
        } => match position {
            UpdatePosition::Prefix => write!(f, [operator, place])?,
            UpdatePosition::Postfix => write!(f, [place, operator])?,
        },
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let precedence = operator.precedence();
            let left_precedence = if *operator == BinaryOperator::Exponent {
                Precedence::Postfix
            } else {
                precedence
            };
            let right_precedence = if *operator == BinaryOperator::Exponent {
                precedence
            } else {
                precedence.tighter()
            };

            format_coalesce_operand(*left, *operator, left_precedence, f)?;
            write!(f, [space(), operator, space()])?;
            format_coalesce_operand(*right, *operator, right_precedence, f)?;
        }
        Expression::Assign { left, right } => {
            write!(f, [left])?;
            write!(f, [space(), token("="), space()])?;
            format_expression_id_with_precedence(*right, Precedence::Assignment, f)?;
        }
        Expression::AssignBinary {
            left,
            operator,
            right,
        } => {
            write!(f, [left])?;
            write!(f, [space(), operator, space()])?;
            format_expression_id_with_precedence(*right, Precedence::Assignment, f)?;
        }
        Expression::PrivateIn { identifier, object } => {
            write!(f, [token("#"), identifier, space(), Keyword::In, space()])?;
            format_expression_id_with_precedence(*object, Precedence::Compare.tighter(), f)?;
        }
        Expression::Member {
            object,
            property,
            is_optional,
        } => {
            let needs_extra_dot = format_member_object(*object, *is_optional, f)?;
            if needs_extra_dot {
                write!(f, [token(".")])?;
            }
            write!(f, [token(if *is_optional { "?." } else { "." })])?;
            write!(f, [property])?;
        }
        Expression::PrivateMember { object, property } => {
            let needs_extra_dot = format_member_object(*object, false, f)?;
            if needs_extra_dot {
                write!(f, [token(".")])?;
            }
            write!(f, [token("."), token("#"), property])?;
        }
        Expression::Index {
            left,
            right,
            is_optional,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Call, f)?;
            write!(f, [token(if *is_optional { "?.[" } else { "[" })])?;
            format_expression_id_with_precedence(*right, Precedence::Lowest, f)?;
            write!(f, [token("]")])?;
        }
        Expression::Call {
            left,
            arguments,
            is_optional,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Call, f)?;
            let open = if *is_optional { "?.(" } else { "(" };
            let mut arguments = delimited(open, ")", ",", arguments);
            arguments.without_trailing_separator();
            write!(f, [arguments])?;
        }
        Expression::ImportCall { specifier, options } => {
            write!(f, [token("import"), token("(")])?;
            format_expression_id_with_precedence(*specifier, Precedence::Assignment, f)?;
            if let Some(options) = options {
                write!(f, [token(","), space()])?;
                format_expression_id_with_precedence(*options, Precedence::Assignment, f)?;
            }
            write!(f, [token(")")])?;
        }
        Expression::New { left, arguments } => {
            write!(f, [token("new"), space()])?;
            let needs_parentheses = f
                .context()
                .tree
                .get(*left)
                .is_optional_chain(f.context().tree);
            if needs_parentheses {
                write!(f, [token("(")])?;
            }
            format_expression_id_with_precedence(*left, Precedence::Member, f)?;
            if needs_parentheses {
                write!(f, [token(")")])?;
            }

            let mut arguments = delimited("(", ")", ",", arguments);
            arguments.without_trailing_separator();
            write!(f, [arguments])?;
        }
        Expression::IfTernary {
            condition,
            then_expression,
            else_expression,
        } => {
            format_expression_id_with_precedence(*condition, Precedence::Conditional.tighter(), f)?;
            write!(f, [space(), token("?"), space()])?;
            format_expression_id_with_precedence(*then_expression, Precedence::Assignment, f)?;
            write!(f, [space(), token(":"), space()])?;

            format_expression_id_with_precedence(*else_expression, Precedence::Assignment, f)?;
        }
    }

    // outer wrap
    if needs_wrap {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one expression id with one required parent precedence.
pub(crate) fn format_expression_id_with_precedence<'ast>(
    expression_id: LocalNodeId<Expression>,
    parent_precedence: Precedence,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);

    let provenance = f.context().tree.provenance(expression_id);

    format_attributed(provenance, None, f, |f| {
        format_expression_with_precedence(expression, parent_precedence, f)
    })
}

/// Format one operand under the nullish coalescing grammar restriction.
fn format_coalesce_operand<'ast>(
    id: LocalNodeId<Expression>,
    parent: BinaryOperator,
    precedence: Precedence,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(id);
    let needs_parentheses = parent == BinaryOperator::Coalesce
        && matches!(
            expression,
            Expression::Binary {
                operator: BinaryOperator::And | BinaryOperator::Or,
                ..
            }
        );

    if needs_parentheses {
        write!(f, [token("(")])?;
    }
    format_expression_id_with_precedence(id, precedence, f)?;
    if needs_parentheses {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one member object and report whether a decimal member needs another dot.
fn format_member_object<'ast>(
    id: LocalNodeId<Expression>,
    is_optional: bool,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<bool> {
    let expression = f.context().tree.get(id);
    format_expression_id_with_precedence(id, Precedence::Call, f)?;

    let needs_extra_dot = expression.needs_decimal_member_dot(is_optional);

    Ok(needs_extra_dot)
}
