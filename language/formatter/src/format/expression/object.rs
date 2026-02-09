use super::*;
use crate::collection::{collection_nodes_have_annotations, collection_nodes_have_newline};
use destack_fir::{format_args, write};

/// Format boundary comments for array-like structures.
pub(super) fn format_boundary_comment_array<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let (Some(first_element), Some(last_element)) = (elements.first(), elements.last()) else {
        write!(f, [token("[]")])?;
        return Ok(());
    };

    let first_span = f.context().get_span(*first_element);
    let last_span = f.context().get_span(*last_element);
    if first_span.file != last_span.file || first_span.start >= last_span.end {
        let fallback_elements = elements.to_vec();
        write!(
            f,
            [list_like("[", "]", ",", &fallback_elements).as_collection()]
        )?;
        return Ok(());
    }

    let value_span = Span::new(first_span.file, first_span.start, last_span.end);
    let value_source = f.context().get_span_str(value_span);
    let value_source = value_source.trim();
    let needs_trailing_comma = matches!(
        f.context().options.trailing_comma,
        TrailingComma::All | TrailingComma::Es5
    );

    let body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [f.context().any_prefix_annotations(*first_element)])?;
        write!(f, [text(value_source)])?;
        if needs_trailing_comma {
            write!(f, [token(",")])?;
        }
        write!(
            f,
            [f.context().any_infix_or_postfix_annotations(*last_element)]
        )?;
        Ok(())
    });

    write!(
        f,
        [group(&format_args![
            token("["),
            hard_line_break(),
            block_indent(&body),
            hard_line_break(),
            token("]")
        ])]
    )
}

/// Whether an expression is used as the left side of an assignment.
pub(super) fn is_assignment_left_target(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_expression_id = expression_id;
    while let Some((ancestor_id, ancestor_type)) = context.get_parent(current_expression_id) {
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
    let Some((field_id, field_type)) = context.get_parent(expression_id) else {
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

    let Some((pattern_id, pattern_type)) = context.get_parent(field_id) else {
        return false;
    };
    if pattern_type != NodeType::Pattern {
        return false;
    }

    let pattern_id = LocalNodeId::<Pattern>::new(pattern_id);
    context.has_newline(context.get_span(pattern_id))
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

    let object_span = context.get_span(expression_id);
    let first_property_span = context.get_span(*first_property_id);
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

/// Format a struct literal.
/// Format a struct literal expression.
#[inline]
pub(crate) fn format_struct_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    ty: &Option<LocalNodeId<Expression>>,
    properties_ids: &Vec<LocalNodeId<Property>>,
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
    let span = f.context().get_span(expression_id);
    let has_newline_in_source = f.context().has_newline(span);
    let is_typescript = f.context().options.language_type.is_typescript();
    let is_static_type_argument = is_static_type_argument_context(f.context(), expression_id);
    let in_type_context =
        is_typescript && (is_type_context(f.context(), expression_id) || is_static_type_argument);
    let has_leading_newline_before_first_property =
        object_has_leading_newline_before_first_property(
            f.context(),
            expression_id,
            properties_ids,
        );
    let keep_newline =
        has_leading_newline_before_first_property || (has_newline_in_source && in_type_context);
    let property_has_newline = collection_nodes_have_newline(f.context(), properties_ids);

    let comment_tokens = collect_comment_tokens(f.context());
    let has_ignore_ranges = !properties_ids.is_empty()
        && properties_ids.iter().any(|property_id| {
            ignore_range_for_node(f.context(), *property_id, &comment_tokens).is_some()
        });

    // only force expand for methods, annotations, comments, or explicit newlines
    // otherwise let best_fitting decide based on line width
    let has_comments = span_has_comment(f.context(), span)
        || properties_ids
            .iter()
            .any(|property_id| span_has_comment(f.context(), f.context().get_span(*property_id)));
    let keep_single_inline_comment_object =
        has_comments && properties_ids.len() == 1 && !has_newline_in_source;
    let keep_single_inline_annotated_object =
        has_annotations && properties_ids.len() == 1 && !has_newline_in_source;
    let has_complex_property = properties_ids
        .iter()
        .copied()
        .any(|property_id| property_has_complex_value(f.context(), property_id));
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
    let keep_inline_assignment_target_commented_object =
        is_assignment_target && has_comments && !has_newline_in_source && properties_ids.len() <= 2;
    let keep_inline_assignment_target_annotated_object = is_assignment_target
        && has_annotations
        && !has_newline_in_source
        && properties_ids.len() <= 2;
    let is_complex_assignment_target = properties_ids.len() > 2 && is_assignment_target;
    let must_expand = has_methods
        || (has_annotations
            && !keep_single_inline_annotated_object
            && !keep_inline_assignment_target_annotated_object)
        || keep_newline
        || keep_complex_newline
        || expand_multiline_pattern_default
        || property_has_newline
        || (has_comments
            && !keep_single_inline_comment_object
            && !keep_inline_assignment_target_commented_object)
        || has_complex_static_type_argument_property
        || should_preserve_tree_attribute_multiline
        || is_complex_assignment_target;
    let separator = if in_type_context { ";" } else { "," };

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
        && properties_ids.len() == 1
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
