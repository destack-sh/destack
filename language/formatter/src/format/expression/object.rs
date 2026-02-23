use crate::format::collection::{collection_nodes_have_annotations, collection_nodes_have_newline};
use crate::format::directive::any_ignore_range_for_nodes;
use crate::format::expression::{
    Argument, DestackFormatContext, DestackFormatter, Expression, FormatResult, LocalNodeId,
    NodeType, Pattern, PatternField, Property, SmallVec, Span, TrailingComma, block_indent,
    format_block_of_properties, format_with, group, hard_line_break, if_group_breaks,
    is_tree_attribute_expression, list_like, property_has_complex_type_value,
    property_has_complex_value, soft_block_indent, soft_line_break_or_space, space, token,
};
use crate::format::operator::{
    is_parameter_type_annotation, is_static_type_argument_context, is_type_context,
};
use destack_fir::format::Buffer;
use destack_fir::{format_args, write};

// object literal shape thresholds
const SINGLE_PROPERTY_COUNT: usize = 1;
const INLINE_ASSIGNMENT_TARGET_MAX_PROPERTIES: usize = 2;
const COMPLEX_ASSIGNMENT_TARGET_MIN_PROPERTIES: usize = 3;

/// Format boundary comments for array-like structures.
pub(crate) fn format_boundary_comment_array<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if elements.is_empty() {
        write!(f, [token("[]")])?;
        return Ok(());
    }

    let should_add_trailing_separator = f.context().options.trailing_comma != TrailingComma::None;
    let body = format_with(|f| {
        for (index, element_id) in elements.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            write!(f, [element_id])?;
        }

        if should_add_trailing_separator {
            write!(f, [token(",")])?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_args![
            token("["),
            soft_block_indent(&body),
            token("]")
        ])]
    )
}

/// Format fill-candidate arrays with the shared fill layout.
pub(crate) fn format_fill_array<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if elements.is_empty() {
        write!(f, [token("[]")])?;
        return Ok(());
    }

    let should_add_trailing_separator = f.context().options.trailing_comma != TrailingComma::None;
    let body = format_with(|f| {
        let mut fill = f.fill();
        for (index, element_id) in elements.iter().copied().enumerate() {
            let separator = format_with(|f| {
                if index == 0 {
                    return Ok(());
                }
                write!(f, [token(","), soft_line_break_or_space()])
            });
            fill.entry(&separator, &element_id);
        }
        fill.finish()?;

        if should_add_trailing_separator {
            write!(f, [if_group_breaks(&token(","))])?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_args![
            token("["),
            soft_block_indent(&body),
            token("]")
        ])]
    )
}

/// Whether an expression is used as the left side of an assignment.
pub(crate) fn is_assignment_left_target(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_expression_id = expression_id;
    while let Some((ancestor_id, ancestor_type)) = context.parent(current_expression_id) {
        if ancestor_type != NodeType::Expression {
            return false;
        }

        let ancestor_expression_id = LocalNodeId::<Expression>::new(ancestor_id);
        match context.tree.get(ancestor_expression_id) {
            Expression::Assign { left, .. } => return left.id == current_expression_id.id,
            Expression::Await { expression }
            | Expression::AwaitMaybe { expression }
            | Expression::Parenthesized { expression }
                if expression.id == current_expression_id.id =>
            {
                current_expression_id = ancestor_expression_id;
            }
            Expression::Statement(inner_expression_id)
                if inner_expression_id.id == current_expression_id.id =>
            {
                current_expression_id = ancestor_expression_id;
            }
            _ => return false,
        }
    }

    false
}

/// Decide whether an object literal is the default value of a multiline pattern field.
fn is_multiline_pattern_field_default_object(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((field_id, field_type)) = context.parent(expression_id) else {
        return false;
    };
    if field_type != NodeType::PatternField {
        return false;
    }

    let field_id = LocalNodeId::<PatternField>::new(field_id);
    let is_default_value = match context.tree.get(field_id) {
        PatternField::Named { default, .. }
        | PatternField::Computed { default, .. }
        | PatternField::Alias { default, .. } => {
            default.is_some_and(|id| id.id == expression_id.id)
        }
        _ => false,
    };
    if !is_default_value {
        return false;
    }

    let Some((pattern_id, pattern_type)) = context.parent(field_id) else {
        return false;
    };
    if pattern_type != NodeType::Pattern {
        return false;
    }

    let pattern_id = LocalNodeId::<Pattern>::new(pattern_id);
    context.node_has_newline(pattern_id)
}

/// Return whether an object literal has a source newline immediately after `{`.
fn object_has_leading_newline_before_first_property(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    properties_ids: &[LocalNodeId<Property>],
) -> bool {
    let Some(first_property_id) = properties_ids.first() else {
        return false;
    };

    let object_span = context.span(expression_id);
    let first_property_span = context.span(*first_property_id);
    if object_span.file != first_property_span.file
        || object_span.start >= first_property_span.start
    {
        return false;
    }

    context.has_newline(Span::new(
        object_span.file,
        object_span.start,
        first_property_span.start,
    ))
}

/// Resolve the source separator style for type-literal object members.
fn type_member_separator(
    _context: &DestackFormatContext<'_>,
    _properties_ids: &[LocalNodeId<Property>],
) -> &'static str {
    ";"
}

/// Format a struct literal expression.
#[inline]
pub(crate) fn format_struct_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    ty: &Option<LocalNodeId<Expression>>,
    properties_ids: &[LocalNodeId<Property>],
) -> FormatResult<()> {
    if let Some(ty) = ty {
        write!(f, [ty, space()])?;
    }

    let properties = properties_ids
        .iter()
        .map(|property| f.context().tree.get(*property))
        .collect::<SmallVec<[_; 3]>>();

    // check for conditions that require expansion
    let has_methods = properties
        .iter()
        .any(|property| matches!(property, Property::Method { body: Some(_), .. }));
    let has_annotations = f.context().has_infix_annotation(expression_id)
        || collection_nodes_have_annotations(f.context(), properties_ids);
    let span = f.context().span(expression_id);
    let has_newline_in_source = f.context().has_newline(span);
    let is_static_type_argument = is_static_type_argument_context(f.context(), expression_id);
    let is_type_position = is_type_context(f.context(), expression_id);
    let in_type_context = is_type_position || is_static_type_argument;
    let has_leading_newline_before_first_property =
        object_has_leading_newline_before_first_property(
            f.context(),
            expression_id,
            properties_ids,
        );
    let keep_newline =
        has_leading_newline_before_first_property || (has_newline_in_source && in_type_context);
    let property_has_newline = collection_nodes_have_newline(f.context(), properties_ids);

    let comment_tokens = f.context().comment_tokens();
    let has_ignore_ranges = !properties_ids.is_empty()
        && any_ignore_range_for_nodes(f.context(), properties_ids, comment_tokens);

    // only force expand for methods, annotations, comments, or explicit newlines
    // otherwise let best_fitting decide based on line width
    let has_comments = properties_ids
        .iter()
        .any(|property_id| f.context().has_comment(f.context().span(*property_id)));
    let keep_single_inline_comment_object =
        has_comments && properties_ids.len() == SINGLE_PROPERTY_COUNT && !has_newline_in_source;
    let keep_single_inline_annotated_object =
        has_annotations && properties_ids.len() == SINGLE_PROPERTY_COUNT && !has_newline_in_source;
    let has_complex_property = properties_ids
        .iter()
        .copied()
        .any(|property_id| property_has_complex_value(f.context(), property_id));
    let keep_inline_short_annotated_object = has_annotations
        && !has_newline_in_source
        && !in_type_context
        && properties_ids.len() <= INLINE_ASSIGNMENT_TARGET_MAX_PROPERTIES;
    let keep_complex_newline = f.context().has_newline(span) && has_complex_property;
    let expand_multiline_pattern_default =
        is_multiline_pattern_field_default_object(f.context(), expression_id);
    let has_complex_static_type_argument_property = is_static_type_argument
        && properties_ids
            .iter()
            .copied()
            .any(|property_id| property_has_complex_type_value(f.context(), property_id));
    let should_preserve_tree_attribute_multiline =
        is_tree_attribute_expression(f.context(), expression_id)
            && has_newline_in_source
            && !properties_ids.is_empty();
    let is_assignment_target = is_assignment_left_target(f.context(), expression_id);
    let keep_inline_assignment_target_commented_object = is_assignment_target
        && has_comments
        && !has_newline_in_source
        && properties_ids.len() <= INLINE_ASSIGNMENT_TARGET_MAX_PROPERTIES;
    let keep_inline_assignment_target_annotated_object = is_assignment_target
        && has_annotations
        && !has_newline_in_source
        && properties_ids.len() <= INLINE_ASSIGNMENT_TARGET_MAX_PROPERTIES;
    let is_complex_assignment_target =
        properties_ids.len() >= COMPLEX_ASSIGNMENT_TARGET_MIN_PROPERTIES && is_assignment_target;
    let mut must_expand = has_methods
        || (has_annotations
            && !keep_inline_short_annotated_object
            && !keep_single_inline_annotated_object
            && !keep_inline_assignment_target_annotated_object)
        || keep_newline
        || keep_complex_newline
        || expand_multiline_pattern_default
        || property_has_newline
        || (has_comments
            && has_newline_in_source
            && !keep_single_inline_comment_object
            && !keep_inline_assignment_target_commented_object)
        || has_complex_static_type_argument_property
        || should_preserve_tree_attribute_multiline
        || is_complex_assignment_target;
    if keep_inline_short_annotated_object {
        must_expand = false;
    }
    let separator = if in_type_context {
        if f.context().options.language_type.is_destack() {
            type_member_separator(f.context(), properties_ids)
        } else {
            ";"
        }
    } else {
        ","
    };

    if has_ignore_ranges {
        write!(
            f,
            [format_args![
                token("{"),
                hard_line_break(),
                block_indent(&format_with(|f| {
                    format_block_of_properties(f, properties_ids, separator)
                })),
                hard_line_break(),
                token("}")
            ]]
        )?;
        return Ok(());
    }

    if in_type_context && must_expand {
        write!(
            f,
            [format_args![
                token("{"),
                hard_line_break(),
                block_indent(&format_with(|f| {
                    format_block_of_properties(f, properties_ids, separator)
                })),
                hard_line_break(),
                token("}")
            ]]
        )?;
        return Ok(());
    }

    let should_inline_parameter_type_literal = in_type_context
        && is_parameter_type_annotation(f.context(), expression_id)
        && properties_ids.len() == SINGLE_PROPERTY_COUNT
        && !has_methods
        && !has_annotations
        && !has_comments
        && !has_newline_in_source
        && !property_has_newline;
    if should_inline_parameter_type_literal {
        let include_space = f.context().options.bracket_spacing;
        if include_space {
            write!(
                f,
                [token("{"), space(), properties_ids[0], space(), token("}")]
            )?;
        } else {
            write!(f, [token("{"), properties_ids[0], token("}")])?;
        }
        return Ok(());
    }

    let mut list = list_like("{", "}", separator, properties_ids);
    list.as_collection()
        .include_space()
        .should_expand(must_expand);
    let ends_with_spread = properties_ids.last().is_some_and(|property_id| {
        matches!(f.context().tree.get(*property_id), Property::Spread { .. })
    });
    if is_assignment_target && ends_with_spread {
        list.disallow_trailing_separator();
    }

    write!(f, [list])?;
    Ok(())
}
