use crate::TsppFormatContext;
use crate::chain::transparent_inner_expression;
use crate::tree::child::tree_expression_contains_callback_break;
use tspp_dir::{Argument, Expression, LocalNodeId, Tree, TreeAttribute};

/// Get the value expression for any call argument variant.
pub(crate) fn argument_value_id(
    tree: &Tree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Positional { value } | Argument::Spread { value } => Some(*value),
        Argument::Elision | Argument::Error => None,
    }
}

/// Return an argument value expression with transparent wrappers removed.
pub(crate) fn argument_transparent_value_id(
    context: &TsppFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    argument_value_id(context.tree, argument_id)
        .map(|value_id| transparent_inner_expression(context, value_id))
}

/// Get the value expression for any tree attribute value variant.
pub(crate) fn tree_attribute_value_id(
    tree: &Tree,
    attribute_id: LocalNodeId<TreeAttribute>,
) -> Option<LocalNodeId<Expression>> {
    tree.get(attribute_id).value()
}

/// Decide whether tree attributes should force the element to break.
pub(crate) fn should_force_break_tree_attributes(
    context: &TsppFormatContext<'_>,
    attributes: &[LocalNodeId<TreeAttribute>],
) -> bool {
    let tree = context.tree;

    // comments on attributes force a break
    if attributes
        .iter()
        .copied()
        .any(|attribute_id| context.has_annotation(attribute_id))
    {
        return true;
    }

    for attribute_id in attributes {
        let Some(value_id) = tree_attribute_value_id(tree, *attribute_id) else {
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
        if let Expression::TreeExpression { children, .. } = tree.get(value_id)
            && children
                .as_ref()
                .is_some_and(|children| !children.is_empty())
        {
            return true;
        }
    }

    false
}
