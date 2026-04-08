use super::transparent_inner_expression;
use crate::DestackFormatContext;
use destack_ast::{Argument, Expression, Key, LocalNodeId, Property, ScalarLiteral, UnaryOperator};

const MAX_SIMPLE_ARGUMENT_DEPTH: u8 = 2;

/// Return whether one expression is simple enough for member-chain and call heuristics.
pub(crate) fn expression_is_simple(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    expression_is_simple_impl(context, expression_id, 0)
}

/// Return whether one expression is simple enough at one recursion depth.
pub(crate) fn expression_is_simple_with_depth(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    depth: u8,
) -> bool {
    expression_is_simple_impl(context, expression_id, depth)
}

/// Return whether one argument is simple enough at one recursion depth.
fn argument_is_simple_impl(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    depth: u8,
) -> bool {
    if depth >= MAX_SIMPLE_ARGUMENT_DEPTH {
        return false;
    }

    match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. } => expression_is_simple_impl(context, *value, depth),
        Argument::Spread { .. } | Argument::Error => false,
    }
}

/// Return whether one expression is simple enough at one recursion depth.
fn expression_is_simple_impl(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    depth: u8,
) -> bool {
    if depth >= MAX_SIMPLE_ARGUMENT_DEPTH {
        return false;
    }

    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) => {
            context.strings.get(*content).chars().count() <= 5
        }
        Expression::ScalarLiteral(_)
        | Expression::Identifier { .. }
        | Expression::PrivateIdentifier { .. }
        | Expression::ImportMeta
        | Expression::NewTarget
        | Expression::This
        | Expression::Super => true,
        Expression::TemplateExpression { value } => {
            template_literal_is_simple(context, value, depth + 1)
        }
        Expression::ObjectExpression { ty, properties } => {
            ty.is_none() && object_expression_is_simple(context, properties, depth)
        }
        Expression::ArrayExpression { elements } => {
            array_expression_is_simple(context, elements, depth)
        }
        Expression::Unary { operator, right } => {
            matches!(
                operator,
                UnaryOperator::Not
                    | UnaryOperator::Negate
                    | UnaryOperator::Plus
                    | UnaryOperator::ElementwiseNot
                    | UnaryOperator::PreIncrement
                    | UnaryOperator::PreDecrement
                    | UnaryOperator::PostIncrement
                    | UnaryOperator::PostDecrement
            ) && expression_is_simple_impl(context, *right, depth)
        }
        Expression::Must { left, .. } | Expression::Maybe { left, .. } => {
            expression_is_simple_impl(context, *left, depth)
        }
        Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            expression_is_simple_impl(context, *left, depth)
                && static_arguments.as_deref().is_none_or(|arguments| {
                    arguments
                        .iter()
                        .copied()
                        .all(|argument_id| argument_is_simple_impl(context, argument_id, depth + 1))
                })
        }
        Expression::Index { left, index, .. } => {
            expression_is_simple_impl(context, *left, depth)
                && index.is_some_and(|index_id| expression_is_simple_impl(context, index_id, depth))
        }
        Expression::QualifiedReference {
            static_arguments, ..
        } => static_arguments.as_deref().is_none_or(|arguments| {
            arguments
                .iter()
                .copied()
                .all(|argument_id| argument_is_simple_impl(context, argument_id, depth + 1))
        }),
        Expression::Call {
            left,
            static_arguments,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            expression_is_simple_impl(context, *left, depth)
                && static_arguments.as_deref().is_none_or(|arguments| {
                    arguments
                        .iter()
                        .copied()
                        .all(|argument_id| argument_is_simple_impl(context, argument_id, depth + 1))
                })
                && dynamic_arguments.len() + usize::from(depth) <= 2
                && dynamic_arguments
                    .iter()
                    .copied()
                    .all(|argument_id| argument_is_simple_impl(context, argument_id, depth + 1))
        }
        Expression::Import {
            source: destack_ast::ImportSource::ImportCall,
            attributes: None,
            arguments: Some(arguments),
            ..
        } => {
            depth < MAX_SIMPLE_ARGUMENT_DEPTH
                && arguments
                    .iter()
                    .copied()
                    .all(|argument_id| argument_is_simple_impl(context, argument_id, depth + 1))
        }
        _ => false,
    }
}

/// Return whether one template literal is simple at one recursion depth.
fn template_literal_is_simple(
    context: &DestackFormatContext<'_>,
    template: &destack_ast::TemplateLiteral,
    depth: u8,
) -> bool {
    match template {
        destack_ast::TemplateLiteral::String { string } => {
            !context.strings.get(*string).contains('\n')
        }
        destack_ast::TemplateLiteral::InterpolatedString { strings, arguments } => {
            strings
                .iter()
                .all(|string| !context.strings.get(*string).contains('\n'))
                && arguments
                    .iter()
                    .copied()
                    .all(|argument_id| argument_is_simple_impl(context, argument_id, depth))
        }
    }
}

/// Return whether one object expression is simple at one recursion depth.
fn object_expression_is_simple(
    context: &DestackFormatContext<'_>,
    properties: &[LocalNodeId<Property>],
    depth: u8,
) -> bool {
    properties
        .iter()
        .copied()
        .all(|property_id| property_is_simple(context, property_id, depth + 1))
}

/// Return whether one property is simple at one recursion depth.
fn property_is_simple(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
    depth: u8,
) -> bool {
    match context.tree.get(property_id) {
        Property::Field {
            modifiers,
            key,
            value,
            default,
        } => {
            modifiers.is_none()
                && default.is_none()
                && key.is_some_and(|key| key_is_simple(&key))
                && value.is_none_or(|value_id| expression_is_simple_impl(context, value_id, depth))
        }
        Property::Method { .. } | Property::Spread { .. } | Property::Error => false,
    }
}

/// Return whether one property key is simple.
fn key_is_simple(key: &Key) -> bool {
    matches!(key, Key::Name(_) | Key::Private(_))
}

/// Return whether one array expression is simple at one recursion depth.
fn array_expression_is_simple(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
    depth: u8,
) -> bool {
    elements
        .iter()
        .copied()
        .all(|argument_id| argument_is_simple_impl(context, argument_id, depth + 1))
}
