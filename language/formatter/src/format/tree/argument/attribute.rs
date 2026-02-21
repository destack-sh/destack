use super::core::tree_attribute_value_id;
use crate::collection::{collection_nodes_have_annotations, collection_value_should_force_break};
use crate::expression::{is_complex_expression, is_expression_breakable, is_trivial_expression};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Argument, Expression, LocalNodeId, Property};
use destack_fir::format::{Buffer, Format};
use destack_fir::prelude::{
    FormatResult, block_indent, format_with, group, if_group_breaks, soft_line_break_or_space,
    space, token,
};
use destack_fir::{format_args, write};

pub(crate) fn property_has_complex_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    // annotations on the property force complexity
    if context.has_annotation(property_id) {
        return true;
    }

    let property = tree.get(property_id);

    // field values and defaults can be complex
    if let Property::Field { value, default, .. } = property {
        // inspect the field value
        let value_is_complex = value.is_some_and(|value_id| {
            let value_expr = tree.get(value_id);
            is_complex_expression(tree, value_expr) || context.has_annotation(value_id)
        });

        // inspect the field default
        let default_is_complex = default.is_some_and(|default_id| {
            let default_expr = tree.get(default_id);
            is_complex_expression(tree, default_expr) || context.has_annotation(default_id)
        });

        return value_is_complex || default_is_complex;
    }

    // methods with bodies are always complex in object literals
    if let Property::Method { body, .. } = property {
        return body.is_some();
    }

    // spread properties inherit complexity from their value
    if let Property::Spread { value, .. } = property {
        let value_expr = tree.get(*value);
        return is_complex_expression(tree, value_expr) || context.has_annotation(*value);
    }

    false
}

/// Check whether a property contains a complex type value.
pub(crate) fn property_has_complex_type_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    if context.has_annotation(property_id) {
        return true;
    }

    let is_complex_type_expression = |expression_id: LocalNodeId<Expression>| {
        let expression = tree.get(expression_id);
        context.has_annotation(expression_id)
            || is_expression_breakable(tree, expression)
            || !is_trivial_expression(tree, expression)
    };

    match tree.get(property_id) {
        Property::Field { value, default, .. } => {
            value.is_some_and(is_complex_type_expression)
                || default.is_some_and(is_complex_type_expression)
        }
        Property::Method { body, .. } => body.is_some(),
        Property::Spread { value, .. } => is_complex_type_expression(*value),
    }
}

/// Decide whether tree attributes should force the element to break.
pub(crate) fn should_force_break_tree_attributes(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    let tree = context.tree;

    // comments on attributes force a break
    if collection_nodes_have_annotations(context, arguments) {
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

        // preserve explicit multiline attribute values
        let value_span = context.span(value_id);
        if context.has_newline(value_span) {
            return true;
        }

        // collect value signals for complexity checks
        let value_expr = tree.get(value_id);

        // complex object and array values should break the element
        match value_expr {
            Expression::ObjectExpression { properties, .. } => {
                // collect object signals
                let has_many_properties = properties.len() > 1;
                let has_complex_property = properties
                    .iter()
                    .copied()
                    .any(|property_id| property_has_complex_value(context, property_id));

                // break when the object is clearly complex
                let should_break_object =
                    collection_value_should_force_break(has_many_properties, has_complex_property);

                if should_break_object {
                    return true;
                }
            }
            Expression::ArrayExpression { elements } => {
                // collect array signals
                let has_many_elements = elements.len() > 1;
                let has_complex_element = elements.iter().any(|element_id| {
                    // annotations on the element force complexity
                    let element_has_annotation = context.has_annotation(*element_id);

                    // inspect the element value when present
                    let element_value_is_complex = tree_attribute_value_id(tree, *element_id)
                        .is_some_and(|element_value_id| {
                            let element_expr = tree.get(element_value_id);
                            is_complex_expression(tree, element_expr)
                                || context.has_annotation(element_value_id)
                        });

                    element_has_annotation || element_value_is_complex
                });

                // break when the array is clearly complex
                let should_break_array =
                    collection_value_should_force_break(has_many_elements, has_complex_element);

                if should_break_array {
                    return true;
                }
            }
            Expression::TreeExpression { elements, .. } => {
                // nested trees with children force a break
                let has_children = elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty());

                if has_children {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

/// Check if an expression is huggable with the given configuration.
/// Return whether an expression is huggable in JSX position.
#[inline]
fn format_tree_attribute_inline_or_hugged<'ast, InlineDoc, HuggedDoc>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    inline_format: InlineDoc,
    hugged_format: HuggedDoc,
) -> FormatResult<()>
where
    InlineDoc: Format<DestackFormatContext<'ast>>,
    HuggedDoc: Format<DestackFormatContext<'ast>>,
{
    // context-based default selection
    if f.context().has_annotation(value_id) {
        f.context()
            .increment_counter("profile.jsx.attribute.by_context.hug", 1);
        hugged_format.format(f)?;
    } else {
        f.context()
            .increment_counter("profile.jsx.attribute.by_context.inline", 1);
        inline_format.format(f)?;
    }

    Ok(())
}

/// Format a tree attribute object value using inline-or-hugged selection.
fn format_tree_attribute_object_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    ty: Option<LocalNodeId<Expression>>,
    properties: Vec<LocalNodeId<Property>>,
) -> FormatResult<()> {
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("="), token("{"), value_id, token("}")])
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [
                token("="),
                token("{"),
                format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    if let Some(ty) = ty {
                        write!(f, [ty, space()])?;
                    }
                    write!(
                        f,
                        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(
                                f,
                                [
                                    token("{"),
                                    block_indent(&format_with(
                                        |f: &mut DestackFormatter<'ast, '_>| {
                                            f.join_with(&format_args![
                                                token(","),
                                                soft_line_break_or_space()
                                            ])
                                            .entries(&properties)
                                            .finish()?;
                                            write!(f, [if_group_breaks(&token(","))])
                                        }
                                    )),
                                    token("}")
                                ]
                            )
                        }))
                        .should_expand(true)]
                    )
                }),
                token("}")
            ]
        )
    });

    format_tree_attribute_inline_or_hugged(f, value_id, inline_format, hugged_format)
}

/// Format a tree attribute array value using inline-or-hugged selection.
fn format_tree_attribute_array_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    elements: Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("="), token("{"), value_id, token("}")])
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [
                token("="),
                token("{"),
                group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(
                        f,
                        [
                            token("["),
                            block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                f.join_with(&format_args![token(","), soft_line_break_or_space()])
                                    .entries(&elements)
                                    .finish()?;
                                write!(f, [if_group_breaks(&token(","))])
                            })),
                            token("]")
                        ]
                    )
                }))
                .should_expand(true),
                token("}")
            ]
        )
    });

    format_tree_attribute_inline_or_hugged(f, value_id, inline_format, hugged_format)
}

/// Format a tree/JSX attribute value with hugging for objects and arrays.
pub(crate) fn format_tree_attribute_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // tree attribute layout
    let tree = f.context().tree;

    match tree.get(value_id) {
        // object attribute value
        Expression::ObjectExpression { ty, properties } => {
            format_tree_attribute_object_value(f, value_id, *ty, properties.clone())?;
        }

        // array attribute value
        Expression::ArrayExpression { elements } => {
            format_tree_attribute_array_value(f, value_id, elements.clone())?;
        }

        // non-huggable values use regular braced formatting
        _ => {
            write!(f, [token("="), token("{"), value_id, token("}")])?;
        }
    }

    Ok(())
}
