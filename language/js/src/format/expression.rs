use crate::tree::Precedence;
use crate::{
    ArrayElement, Asynchrony, Expression, FunctionCardinality, Keyword, LocalNodeId,
    PostfixPosition,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::NodeSpanType;

use crate::format::argument::{format_type_parameter_list, list_like};
use crate::format::function::format_function_signature_parameters;
use crate::format::literal::{
    format_scalar_literal, format_string_literal_with_source_span, format_template_literal,
};
use crate::{FormatNode, JsFormatter};

impl<'ast> FormatNode<'ast, ArrayElement> for ArrayElement {
    fn format_node(
        &self,
        _node_id: LocalNodeId<ArrayElement>,
        f: &mut JsFormatter<'ast, '_>,
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
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_expression_with_precedence(node_id, self, Precedence::Lowest, f)
    }
}

/// Format one expression with one required parent precedence.
fn format_expression_with_precedence<'ast>(
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
    parent_precedence: Precedence,
    f: &mut JsFormatter<'ast, '_>,
) -> FormatResult<()> {
    let expression = Expression::without_parentheses(f.context().tree, expression);
    let current_precedence = expression.precedence();
    let needs_wrap = current_precedence < parent_precedence;

    // outer wrap
    if needs_wrap {
        write!(f, [token("(")])?;
    }

    if !f.context().include_types() && expression.is_type_only(f.context().tree) {
        return Ok(());
    }

    match expression {
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

            // generic parameters
            if f.context().include_types() && !signature.generic_parameters.is_empty() {
                format_type_parameter_list(&signature.generic_parameters, f)?;
            }

            // parameters
            format_function_signature_parameters(signature, f)?;

            // return type
            if f.context().include_types()
                && let Some(return_type) = signature.return_type
            {
                write!(f, [token(":"), space(), return_type])?;
            }

            // body
            write!(f, [space(), token("=>"), space()])?;
            match body {
                crate::ArrowFunctionBody::Expression(body) => {
                    format_expression_id_with_precedence(*body, Precedence::Assignment, f)?;
                }
                crate::ArrowFunctionBody::Block(body) => {
                    write!(f, [body])?;
                }
            }
        }
        Expression::Path {
            path,
            generic_arguments,
        } => {
            write!(f, [path])?;

            if f.context().include_types() && !generic_arguments.is_empty() {
                write!(f, [list_like("<", ">", ",", generic_arguments)])?;
            }
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
        Expression::NewTarget => {
            write!(f, [Keyword::New, token("."), token("target")])?;
        }
        Expression::PrivateIdentifier { name } => {
            write!(f, [token("#"), *name])?;
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
        Expression::As {
            expression,
            target_type,
        } => {
            format_expression_id_with_precedence(*expression, Precedence::Compare, f)?;
            write!(f, [space(), Keyword::As, space(), target_type])?;
        }
        Expression::Satisfies {
            expression,
            target_type,
        } => {
            format_expression_id_with_precedence(*expression, Precedence::Compare, f)?;
            write!(f, [space(), Keyword::Satisfies, space(), target_type])?;
        }
        Expression::Is { value, target_type } => {
            format_expression_id_with_precedence(*value, Precedence::Compare, f)?;
            write!(f, [space(), Keyword::Is, space(), target_type])?;
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
                let needs_space = matches!(
                    operator,
                    crate::UnaryOperator::Typeof | crate::UnaryOperator::Void
                );

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
            let left_precedence = if *operator == crate::BinaryOperator::Exponent {
                precedence.tighter()
            } else {
                precedence
            };
            let right_precedence = if *operator == crate::BinaryOperator::Exponent {
                precedence
            } else {
                precedence.tighter()
            };

            format_expression_id_with_precedence(*left, left_precedence, f)?;
            write!(f, [space(), operator, space()])?;
            format_expression_id_with_precedence(*right, right_precedence, f)?;
        }
        Expression::Assign { left, right } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;
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
        Expression::Maybe { position, left } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;

            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }

            write!(f, [token("?")])?;
        }
        Expression::Must { position, left } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;

            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }

            write!(f, [token("!")])?;
        }
        Expression::Member {
            left,
            name,
            generic_arguments,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;
            write!(f, [token("."), *name])?;

            if f.context().include_types() && !generic_arguments.is_empty() {
                write!(f, [list_like("<", ">", ",", generic_arguments)])?;
            }
        }
        Expression::PrivateMember {
            left,
            name,
            generic_arguments,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;
            write!(f, [token("."), token("#"), *name])?;

            if f.context().include_types() && !generic_arguments.is_empty() {
                write!(f, [list_like("<", ">", ",", generic_arguments)])?;
            }
        }
        Expression::Index {
            position,
            left,
            right,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;

            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }

            write!(f, [token("[")])?;
            format_expression_id_with_precedence(*right, Precedence::Lowest, f)?;
            write!(f, [token("]")])?;
        }
        Expression::Instantiation {
            left,
            generic_arguments,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;

            if f.context().include_types() {
                write!(f, [list_like("<", ">", ",", generic_arguments)])?;
            }
        }
        Expression::Call {
            position,
            left,
            generic_arguments,
            arguments,
        } => {
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;

            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }

            if f.context().include_types() && !generic_arguments.is_empty() {
                write!(f, [list_like("<", ">", ",", generic_arguments)])?;
            }

            let mut arguments = list_like("(", ")", ",", arguments);
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
            if let Expression::ScalarLiteral {
                value: crate::ScalarLiteral::String(value),
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
        Expression::New {
            left,
            generic_arguments,
            arguments,
        } => {
            write!(f, [token("new"), space()])?;
            format_expression_id_with_precedence(*left, Precedence::Postfix, f)?;

            if f.context().include_types() && !generic_arguments.is_empty() {
                write!(f, [list_like("<", ">", ",", generic_arguments)])?;
            }

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

            if let Some(else_expression) = else_expression {
                format_expression_id_with_precedence(*else_expression, Precedence::Assignment, f)?;
            }
        }
        Expression::Missing => {
            write!(f, [token("/* MISSING */")])?;
        }
        Expression::Stub => {}
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
    f: &mut JsFormatter<'ast, '_>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);

    format_expression_with_precedence(expression_id, expression, parent_precedence, f)
}
