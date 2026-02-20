use super::{
    Argument, DestackFormatContext, Expression, LocalNodeId, argument_has_non_blank_annotation,
    argument_value_id, is_expression_breakable, transparent_inner_expression,
};
use crate::operator::expression_static_arguments;

// static argument hugging thresholds
const HUG_STATIC_ARGUMENT_MAX_COUNT: usize = 3;

/// Decide whether static argument lists should expand at the list level.
pub(crate) fn should_expand_static_argument_list(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.len() != 1 {
        return false;
    }

    // keep single direct object-like type arguments hugged as `<{ ... }>`
    let value_id = argument_value_id(context.tree, static_arguments[0]);
    let value_id = transparent_inner_expression(context, value_id);
    if matches!(
        context.tree.get(value_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    ) {
        return false;
    }

    // only expand list-level generic wrappers when source is already multiline and
    // the nested type arguments include object-like forms
    if !context.node_has_newline(value_id) {
        return false;
    }

    expression_static_arguments(context.tree.get(value_id)).is_some_and(|nested_arguments| {
        nested_arguments.iter().copied().any(|nested_argument_id| {
            let nested_value_id = argument_value_id(context.tree, nested_argument_id);
            let nested_value_id = transparent_inner_expression(context, nested_value_id);
            matches!(
                context.tree.get(nested_value_id),
                Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
            )
        })
    })
}

/// Decide whether static argument lists should stay inline regardless of line width.
pub(crate) fn should_hug_static_argument_list(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    static_argument_list_is_hug_safe(context, static_arguments)
}

/// Return whether static arguments are structurally safe for hugged inline formatting.
fn static_argument_list_is_hug_safe(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.is_empty() || static_arguments.len() > HUG_STATIC_ARGUMENT_MAX_COUNT {
        return false;
    }

    static_arguments
        .iter()
        .copied()
        .all(|argument_id| static_argument_is_hug_safe(context, argument_id))
}

/// Return whether one static argument is structurally safe for hugged inline formatting.
fn static_argument_is_hug_safe(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if argument_has_non_blank_annotation(context, argument_id) {
        return false;
    }

    let argument_value_id = argument_value_id(context.tree, argument_id);
    let argument_value_id = transparent_inner_expression(context, argument_value_id);
    if context.node_has_newline(argument_value_id) {
        return false;
    }

    static_argument_expression_is_hug_safe(context, argument_value_id)
}

/// Return whether one static argument expression is structurally safe for hugged inline formatting.
fn static_argument_expression_is_hug_safe(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Path {
            static_arguments, ..
        } => static_arguments.as_deref().is_none_or(|static_arguments| {
            static_argument_list_is_hug_safe(context, static_arguments)
        }),
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
            static_argument_expression_is_hug_safe(context, *left)
                && static_arguments.as_deref().is_none_or(|static_arguments| {
                    static_argument_list_is_hug_safe(context, static_arguments)
                })
        }
        Expression::Index { left, index, .. } => {
            static_argument_expression_is_hug_safe(context, *left)
                && index.is_none_or(|index_id| {
                    static_argument_expression_is_hug_safe(context, index_id)
                })
        }
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            static_argument_expression_is_hug_safe(context, *expression)
        }
        _ => false,
    }
}

/// Decide whether a mapped type should force multiline formatting.
pub(crate) fn should_force_multiline_mapped_type(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(node_id) {
        return true;
    }

    let span = context.span(node_id);
    if context.has_newline(span) {
        return true;
    }

    is_expression_breakable(context.tree, context.tree.get(value_id))
}
