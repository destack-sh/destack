use crate::chain::transparent_inner_expression;
use crate::tree::child::tree_expression_contains_callback_break;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Argument, Expression, LocalNodeId, Tree};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::token;
use destack_fir::write;

/// Get the value expression for any tree attribute argument variant.
pub(crate) fn tree_attribute_value_id(
    tree: &Tree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => Some(*value),
        Argument::Error => None,
    }
}

/// Return an argument value expression with transparent wrappers removed.
pub(crate) fn argument_transparent_value_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    tree_attribute_value_id(context.tree, argument_id)
        .map(|value_id| transparent_inner_expression(context, value_id))
}

/// Decide whether tree attributes should force the element to break.
pub(crate) fn should_force_break_tree_attributes(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    let tree = context.tree;

    // comments on attributes force a break
    if arguments
        .iter()
        .copied()
        .any(|argument_id| context.has_annotation(argument_id))
    {
        return true;
    }

    for argument_id in arguments {
        let Some(value_id) = tree_attribute_value_id(tree, *argument_id) else {
            continue;
        };

        // comments on attribute values force a break
        if context.has_annotation(value_id) {
            return true;
        }

        // callback-bearing expression values should break the opening tag
        if tree_expression_contains_callback_break(context, value_id) {
            return true;
        }

        // nested trees with children force a break
        if let Expression::TreeExpression { elements, .. } = tree.get(value_id)
            && elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty())
        {
            return true;
        }
    }

    false
}

/// Format a tree or JSX attribute value.
pub(crate) fn format_tree_attribute_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [token("="), token("{"), value_id, token("}")])
}
