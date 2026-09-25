use crate::tree::Precedence;
use crate::{
    ArrayElement, ArrowFunctionBody, Asynchrony, BinaryOperator, Expression, Keyword, Literal,
    LocalNodeId, UnaryOperator,
};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::write;
use tspp_source::NodeSpanType;

use crate::format::argument::list_like;
use crate::format::function::format_function_parameters;
use crate::format::literal::{
    format_scalar_literal, format_string_literal_with_source_span, format_template_literal,
};
use crate::{FormatNode, Formatter};

impl<'ast> FormatNode<'ast, ArrayElement> for ArrayElement {
    fn format_node(
        &self,
        _node_id: LocalNodeId<ArrayElement>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            ArrayElement::Expression { value } => {
                write!(f, [value])?;
            }
            ArrayElement::Spread { value } => {
                write!(f, [token("..."), value])?;
            }
            ArrayElement::Elision => {}
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: LocalNodeId<Expression>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_expression_with_precedence(node_id, self, Precedence::Lowest, f)
    }
}

/// Format one expression with one required parent precedence.
fn format_expression_with_precedence<'ast>(
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
    parent_precedence: Precedence,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let expression = Expression::without_parentheses(f.context().tree, expression);
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
            body,
        } => {
            // asynchrony
            if *asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Async, space()])?;
            }

            // parameters
            format_function_parameters(parameters, f)?;

            // body
            write!(f, [space(), token("=>"), space()])?;
            match body {
                ArrowFunctionBody::Expression(body) => {
                    format_expression_id_with_precedence(*body, Precedence::Assignment, f)?;
                }
                ArrowFunctionBody::Block(body) => {
                    write!(f, [body])?;
                }
            }
        }
        Expression::Path { path } => {
            write!(f, [path])?;
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
        Expression::PrivateIdentifier { name } => {
            write!(f, [token("#"), *name])?;
        }
        Expression::Literal { value } => {
            format_scalar_literal(value, f)?;
        }
        Expression::TemplateLiteral { value } => {
            format_template_literal(value, f)?;
        }
        Expression::ArrayLiteral { elements } => {
            write!(f, [list_like("[", "]", ",", elements)])?;
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
            let mut properties = list_like("{", "}", ",", properties);
            properties.include_space();
            write!(f, [properties])?;
        }
        Expression::Parenthesized { .. } => {
            unreachable!("parenthesized expressions are unwrapped")
        }
        Expression::InstanceOf { value, target } => {
            format_expression_id_with_precedence(*value, Precedence::Compare, f)?;
            write!(f, [space(), Keyword::InstanceOf, space()])?;
            format_expression_id_with_precedence(*target, Precedence::Compare.tighter(), f)?;
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
            // operator
            if operator.is_prefix() {
                let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);

                if needs_space {
                    write!(f, [operator, space()])?;
                } else {
                    write!(f, [operator])?;
                }

                format_expression_id_with_precedence(*right, Precedence::Prefix, f)?;
            } else {
                format_expression_id_with_precedence(*right, Precedence::Prefix, f)?;
                write!(f, [operator])?;
            }
        }
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let precedence = operator.precedence();
            let left_precedence = if *operator == BinaryOperator::Exponent {
                precedence.tighter()
            } else {
                precedence
            };
            let right_precedence = if *operator == BinaryOperator::Exponent {
                precedence
            } else {
                precedence.tighter()
            };

            format_expression_id_with_precedence(*left, left_precedence, f)?;
            write!(f, [space(), operator, space()])?;
            format_expression_id_with_precedence(*right, right_precedence, f)?;
        }
        Expression::Assign { left, right } => {
            write!(f, [left])?;
            write!(f, [token("=")])?;
            format_expression_id_with_precedence(*right, Precedence::Assignment, f)?;
        }
        Expression::AssignBinary {
            left,
            operator,
            right,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;
            write!(f, [operator])?;
            format_expression_id_with_precedence(*right, Precedence::Assignment, f)?;
        }
        Expression::Member {
            left,
            name,
            is_optional,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;
            write!(f, [token(if *is_optional { "?." } else { "." }), *name])?;
        }
        Expression::PrivateMember { left, name } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;
            write!(f, [token("."), token("#"), *name])?;
        }
        Expression::Index {
            left,
            right,
            is_optional,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;
            write!(f, [token(if *is_optional { "?.[" } else { "[" })])?;
            format_expression_id_with_precedence(*right, Precedence::Lowest, f)?;
            write!(f, [token("]")])?;
        }
        Expression::Call {
            left,
            arguments,
            is_optional,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;
            let open = if *is_optional { "?.(" } else { "(" };
            let mut arguments = list_like(open, ")", ",", arguments);
            arguments.without_trailing_separator();
            write!(f, [arguments])?;
        }
        Expression::ImportCall {
            target,
            target_module: _,
            arguments,
        } => {
            let target_expression = f.context().tree.get(*target);
            let target_span = f.context().source_part_span(node_id.id, NodeSpanType::Main);

            write!(f, [token("import"), token("(")])?;

            // exact target literal span
            if let Expression::Literal {
                value: Literal::String(value),
            } = target_expression
            {
                format_string_literal_with_source_span(*value, target_span, f)?;
            } else {
                format_expression_id_with_precedence(*target, Precedence::Lowest, f)?;
            }

            if !arguments.is_empty() {
                write!(f, [token(","), space()])?;
                let mut arguments = list_like("", "", ",", arguments);
                arguments.without_trailing_separator();
                write!(f, [arguments])?;
            }

            write!(f, [token(")")])?;
        }
        Expression::New { left, arguments } => {
            write!(f, [token("new"), space()])?;
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;

            let mut arguments = list_like("(", ")", ",", arguments);
            arguments.without_trailing_separator();
            write!(f, [arguments])?;
        }
        Expression::IfTernary {
            condition,
            then_expression,
            else_expression,
        } => {
            format_expression_id_with_precedence(*condition, Precedence::Conditional, f)?;
            write!(f, [token("?")])?;
            format_expression_id_with_precedence(*then_expression, Precedence::Assignment, f)?;
            write!(f, [token(":")])?;

            format_expression_id_with_precedence(*else_expression, Precedence::Assignment, f)?;
        }
        Expression::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    // outer wrap
    if needs_wrap {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one expression id with one required parent precedence.
fn format_expression_id_with_precedence<'ast>(
    expression_id: LocalNodeId<Expression>,
    parent_precedence: Precedence,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);

    format_expression_with_precedence(expression_id, expression, parent_precedence, f)
}
