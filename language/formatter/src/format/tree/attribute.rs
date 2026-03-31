use crate::format::call::expression_has_complex_callback;
use crate::format::chain::transparent_inner_expression;
use crate::format::collection::property_has_complex_value;
use crate::format::expression::is_complex_expression;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Argument, Expression, LocalNodeId};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::token;
use destack_fir::write;

/// Get the value expression for any tree attribute argument variant.
pub(crate) fn tree_attribute_value_id(
    tree: &destack_ast::NodeTree,
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

        // callback-rich expression values should break the opening tag
        if expression_has_complex_callback(context, value_id) {
            return true;
        }

        let value_expression = tree.get(value_id);
        match value_expression {
            // complex object values should break the element
            Expression::ObjectExpression { properties, .. } => {
                let has_many_properties = properties.len() > 1;
                let has_complex_property = properties
                    .iter()
                    .copied()
                    .any(|property_id| property_has_complex_value(context, property_id));

                if has_many_properties && has_complex_property {
                    return true;
                }
            }
            // complex array values should break the element
            Expression::ArrayExpression { elements } => {
                let has_many_elements = elements.len() > 1;
                let has_complex_element = elements.iter().any(|element_id| {
                    let element_has_annotation = context.has_annotation(*element_id);
                    let element_value_is_complex = tree_attribute_value_id(tree, *element_id)
                        .is_some_and(|element_value_id| {
                            let element_expression = tree.get(element_value_id);
                            is_complex_expression(tree, element_expression)
                                || context.has_annotation(element_value_id)
                        });

                    element_has_annotation || element_value_is_complex
                });

                if has_many_elements && has_complex_element {
                    return true;
                }
            }
            // nested trees with children force a break
            Expression::TreeExpression { elements, .. } => {
                if elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty())
                {
                    return true;
                }
            }
            _ => {}
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
