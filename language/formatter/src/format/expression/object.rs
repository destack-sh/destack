use crate::format::annotation::{block_infix_annotations, format_raw_comment};
use crate::format::collection::{
    TrailingSeparator, format_block_nodes_with_ignore_ranges, separated_entries,
};
use crate::format::file::any_ignore_range_for_nodes;
use crate::format::operator::expression_generic_arguments;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Expression, GenericArgument, LocalNodeId, NodeType, Pattern, PatternField, Property,
    TypeExpression,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, if_group_breaks, if_group_fits_on_line,
    soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::TrailingComma;
use smallvec::SmallVec;

// object literal shape thresholds
const SINGLE_PROPERTY_COUNT: usize = 1;
const COMPLEX_DESTRUCTURING_MAX_SIMPLE_PROPERTIES: usize = 2;

/// The top-level layout choices for one struct literal.
#[derive(Clone, Copy, Debug)]
enum StructLiteralLayout {
    Empty,
    Expanded {
        separator: &'static str,
    },
    InlineParameterTypeLiteral {
        property_id: LocalNodeId<Property>,
    },
    Grouped {
        separator: &'static str,
        trailing_separator: TrailingSeparator,
        should_expand: bool,
    },
}

/// Format a block of properties with empty-annotation and ignore-range handling.
fn format_block_of_properties<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    properties: &[LocalNodeId<Property>],
    separator: &'static str,
) -> FormatResult<()> {
    format_block_nodes_with_ignore_ranges(f, properties, |f, property_id| {
        let property = f.context().tree.get(property_id);

        write!(f, [property_id])?;

        if matches!(
            property,
            Property::Field { .. } | Property::Method { .. } | Property::Spread { .. }
        ) {
            write!(f, [token(separator)])?;
        }

        Ok(())
    })
}

/// Write raw comments between the last object property and `}`.
fn write_object_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    properties: &[LocalNodeId<Property>],
) -> FormatResult<()> {
    let Some(last_property_id) = properties.last().copied() else {
        return Ok(());
    };

    let node_span = f.context().span(expression_id);
    let Some(close_brace_token) = f.context().last_non_trivia_token_in_span(node_span) else {
        return Ok(());
    };

    let property_span = f.context().span(last_property_id);
    if property_span.file != close_brace_token.span.file
        || property_span.start >= close_brace_token.span.start
    {
        return Ok(());
    }

    let comment_start = f
        .context()
        .previous_non_trivia_token_before_span(close_brace_token.span)
        .filter(|token| token.span.file == close_brace_token.span.file)
        .map_or(property_span.end, |token| token.span.end);
    if comment_start >= close_brace_token.span.start {
        return Ok(());
    }

    let comment_nodes = {
        let comments = f.context().comments();
        comments
            .comments_in_range(comment_start, close_brace_token.span.start)
            .to_vec()
    };
    if comment_nodes.is_empty() {
        return Ok(());
    }

    let first_comment_span = comment_nodes[0].span;
    let leading_gap = Span::new(node_span.file, comment_start, first_comment_span.start);
    if f.context().has_newline(leading_gap)
        || f.context().span_starts_on_own_line(first_comment_span)
    {
        write!(f, [hard_line_break()])?;
    } else {
        write!(f, [space()])?;
    }

    for (index, comment_id) in comment_nodes.iter().copied().enumerate() {
        let comment_span = comment_id.span;
        format_raw_comment(f, comment_id)?;

        let is_last = index + 1 == comment_nodes.len();
        if !is_last
            || f.context()
                .span_has_newline_before_next_non_whitespace_token(comment_span)
        {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Return whether any property in one collection has annotations.
fn properties_have_annotations(
    context: &DestackFormatContext<'_>,
    property_ids: &[LocalNodeId<Property>],
) -> bool {
    property_ids
        .iter()
        .copied()
        .any(|property_id| context.has_annotation(property_id))
}

/// Return whether any property in one collection spans multiple source lines.
fn properties_have_newline(
    context: &DestackFormatContext<'_>,
    property_ids: &[LocalNodeId<Property>],
) -> bool {
    property_ids
        .iter()
        .copied()
        .any(|property_id| context.node_has_newline(property_id))
}

/// Return whether an expression is the value of a tree attribute argument.
fn is_tree_attribute_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let Some((expression_id, expression_type)) = context.parent_by_id(argument_id) else {
        return false;
    };
    if expression_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::TreeExpression { .. }
    )
}

/// Return whether an expression is the type annotation of a parameter.
pub(crate) fn is_parameter_type_annotation(
    _context: &DestackFormatContext<'_>,
    _expression_id: LocalNodeId<Expression>,
) -> bool {
    false
}

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
            _ => return false,
        }
    }

    false
}

/// Return whether one preserved parenthesized assignment target should expand.
fn object_assignment_target_has_complex_destructuring(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ObjectExpression { properties, .. } => {
            is_assignment_left_target(context, expression_id)
                && properties.len() > COMPLEX_DESTRUCTURING_MAX_SIMPLE_PROPERTIES
                && properties.iter().copied().any(|property_id| {
                    match context.tree.get(property_id) {
                        Property::Field { .. } => false,
                        Property::Method { .. } | Property::Error => true,
                        Property::Spread { .. } => false,
                    }
                })
        }
        _ => false,
    }
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

/// Write one expanded object or struct literal body.
fn write_expanded_struct_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    properties_ids: &[LocalNodeId<Property>],
    separator: &'static str,
) -> FormatResult<()> {
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
    )
}

/// Write one empty struct literal.
fn write_empty_struct_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if f.context().has_infix_annotation(expression_id) {
        write!(
            f,
            [group(&format_args![
                token("{"),
                block_indent(&block_infix_annotations(f.context(), expression_id)),
                hard_line_break(),
                token("}")
            ])]
        )?;
    } else {
        write!(f, [token("{}")])?;
    }

    Ok(())
}

/// Write one inline single-property parameter type literal.
fn write_inline_parameter_type_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    property_id: LocalNodeId<Property>,
) -> FormatResult<()> {
    if f.context().options.bracket_spacing {
        write!(f, [token("{"), space(), property_id, space(), token("}")])
    } else {
        write!(f, [token("{"), property_id, token("}")])
    }
}

/// Write one grouped object or struct literal body.
fn write_grouped_struct_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    properties_ids: &[LocalNodeId<Property>],
    separator: &'static str,
    trailing_separator: TrailingSeparator,
    should_expand: bool,
) -> FormatResult<()> {
    let group_id = f.group_id("object_like");

    write!(
        f,
        [group(&format_args![
            token("{"),
            soft_block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                if f.context().options.bracket_spacing {
                    write!(f, [if_group_fits_on_line(&space())])?;
                }

                write!(
                    f,
                    [separated_entries(
                        separator,
                        properties_ids,
                        trailing_separator,
                        Some(group_id),
                    )]
                )?;
                write_object_trailing_comments(f, expression_id, properties_ids)?;

                if f.context().options.bracket_spacing {
                    write!(f, [if_group_fits_on_line(&space())])?;
                }

                Ok(())
            })),
            token("}")
        ])
        .with_id(Some(group_id))
        .should_expand(should_expand)]
    )
}

/// Return the separator for one struct literal in the current context.
fn struct_literal_separator(
    f: &DestackFormatter<'_, '_>,
    in_type_context: bool,
    properties_ids: &[LocalNodeId<Property>],
) -> &'static str {
    if !in_type_context {
        return ",";
    }

    if f.context().options.language_type.is_destack() {
        return type_member_separator(f.context(), properties_ids);
    }

    ";"
}

/// Decide the top-level layout for one struct literal.
fn struct_literal_layout(
    f: &DestackFormatter<'_, '_>,
    expression_id: LocalNodeId<Expression>,
    properties_ids: &[LocalNodeId<Property>],
) -> StructLiteralLayout {
    if properties_ids.is_empty() {
        return StructLiteralLayout::Empty;
    }

    let properties = properties_ids
        .iter()
        .map(|property| f.context().tree.get(*property))
        .collect::<SmallVec<[_; 3]>>();

    let has_methods = properties
        .iter()
        .any(|property| matches!(property, Property::Method { body: Some(_), .. }));
    let has_annotations = f.context().has_infix_annotation(expression_id)
        || properties_have_annotations(f.context(), properties_ids);
    let has_newline_in_source = f.context().has_newline(f.context().span(expression_id));
    let is_static_type_argument =
        f.context()
            .parent(expression_id)
            .is_some_and(|(generic_argument_id, parent_type)| {
                if parent_type != NodeType::GenericArgument {
                    return false;
                }

                let generic_argument_id = LocalNodeId::<GenericArgument>::new(generic_argument_id);
                let generic_argument_value = match f.context().tree.get(generic_argument_id) {
                    GenericArgument::Value { value } => *value,
                    GenericArgument::Type { .. } => return false,
                    GenericArgument::Error => return false,
                };
                if generic_argument_value.id != expression_id.id {
                    return false;
                }

                let Some((parent_expression_id, expression_type)) =
                    f.context().parent(generic_argument_id)
                else {
                    return false;
                };
                if expression_type != NodeType::Expression {
                    return false;
                }

                let parent_expression_id = LocalNodeId::<Expression>::new(parent_expression_id);
                expression_generic_arguments(f.context().tree.get(parent_expression_id))
                    .is_some_and(|arguments| arguments.contains(&generic_argument_id))
            });
    let in_type_context =
        f.context().is_in_type_expression_root(expression_id) || is_static_type_argument;
    let has_leading_newline_before_first_property =
        object_has_leading_newline_before_first_property(
            f.context(),
            expression_id,
            properties_ids,
        );
    let keep_newline =
        has_leading_newline_before_first_property || (has_newline_in_source && in_type_context);
    let property_has_newline = properties_have_newline(f.context(), properties_ids);
    let comment_tokens = f.context().comment_tokens();
    let has_ignore_ranges = any_ignore_range_for_nodes(f.context(), properties_ids, comment_tokens);
    let has_comments = properties_ids.iter().any(|property_id| {
        let property_span = f.context().span(*property_id);
        !f.context()
            .comments_in_range(property_span.start, property_span.end)
            .is_empty()
    });
    let keep_single_inline_comment_object =
        has_comments && properties_ids.len() == SINGLE_PROPERTY_COUNT && !has_newline_in_source;
    let keep_single_inline_annotated_object =
        has_annotations && properties_ids.len() == SINGLE_PROPERTY_COUNT && !has_newline_in_source;
    let expand_multiline_pattern_default =
        is_multiline_pattern_field_default_object(f.context(), expression_id);
    let should_preserve_tree_attribute_multiline =
        is_tree_attribute_expression(f.context(), expression_id)
            && has_newline_in_source
            && !properties_ids.is_empty();
    let is_assignment_target = is_assignment_left_target(f.context(), expression_id);
    let has_complex_destructuring_assignment_target =
        object_assignment_target_has_complex_destructuring(f.context(), expression_id);
    let should_expand = has_methods
        || (has_annotations && !keep_single_inline_annotated_object)
        || keep_newline
        || expand_multiline_pattern_default
        || property_has_newline
        || (has_comments && has_newline_in_source && !keep_single_inline_comment_object)
        || should_preserve_tree_attribute_multiline
        || has_complex_destructuring_assignment_target
        || (is_assignment_target && has_newline_in_source);
    let separator = struct_literal_separator(f, in_type_context, properties_ids);

    if has_ignore_ranges || (in_type_context && should_expand) {
        return StructLiteralLayout::Expanded { separator };
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
        return StructLiteralLayout::InlineParameterTypeLiteral {
            property_id: properties_ids[0],
        };
    }

    let ends_with_spread = properties_ids.last().is_some_and(|property_id| {
        matches!(f.context().tree.get(*property_id), Property::Spread { .. })
    });
    let allow_trailing_separator = !(is_assignment_target && ends_with_spread);
    let trailing_separator =
        if !allow_trailing_separator || f.context().options.trailing_comma == TrailingComma::None {
            TrailingSeparator::Omit
        } else {
            TrailingSeparator::Allowed
        };

    StructLiteralLayout::Grouped {
        separator,
        trailing_separator,
        should_expand,
    }
}

/// Format a struct literal expression.
#[inline]
pub(crate) fn format_struct_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    ty: &Option<LocalNodeId<TypeExpression>>,
    properties_ids: &[LocalNodeId<Property>],
) -> FormatResult<()> {
    if let Some(ty) = ty {
        write!(f, [ty, space()])?;
    }

    match struct_literal_layout(f, expression_id, properties_ids) {
        StructLiteralLayout::Empty => write_empty_struct_literal(f, expression_id),
        StructLiteralLayout::Expanded { separator } => {
            write_expanded_struct_literal(f, properties_ids, separator)
        }
        StructLiteralLayout::InlineParameterTypeLiteral { property_id } => {
            write_inline_parameter_type_literal(f, property_id)
        }
        StructLiteralLayout::Grouped {
            separator,
            trailing_separator,
            should_expand,
        } => write_grouped_struct_literal(
            f,
            expression_id,
            properties_ids,
            separator,
            trailing_separator,
            should_expand,
        ),
    }
}
