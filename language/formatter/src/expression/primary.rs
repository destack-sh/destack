use super::object::{format_fill_array, format_outer_comment_array, format_struct_literal};
use super::{
    array_elements_are_fill_candidates, array_has_only_outer_comments, is_trivial_argument,
};
use crate::annotation::{
    FormatTrailingComments, block_infix_annotations, format_dangling_comments,
    infix_or_postfix_annotations, postfix_annotations, write_comment_slice,
};
use crate::call::argument::arguments_have_annotations;
use crate::collection::literal::{format_scalar_literal, format_template_literal};
use crate::collection::{TrailingSeparator, separated_entries};
use crate::context::FormatNodeWithoutTrailingComments;
use crate::operator::format_generic_argument_list;
use crate::tree::format_tree_literal_expression;
use crate::{TsppFormatContext, TsppFormatter};
use tspp_dir::{
    Argument, Expression, Keyword, LocalNodeId, TokenSpan, TokenType, Tree, TypeExpression,
};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::{
    block_indent, format_with, group, hard_line_break, soft_block_indent, soft_line_break_or_space,
    space, text, token,
};
use tspp_fir::{format_args, write};
use tspp_repository::TrailingComma;
use tspp_source::Span;

/// Return whether one argument collection is inline between its outer spans.
fn argument_range_is_inline(
    context: &TsppFormatContext<'_>,
    argument_ids: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first_id), Some(last_id)) = (argument_ids.first(), argument_ids.last()) else {
        return false;
    };

    let first_span = context.span(*first_id);
    let last_span = context.span(*last_id);
    if first_span.file != last_span.file || first_span.start >= last_span.end {
        return false;
    }

    !context.has_newline(Span::new(first_span.file, first_span.start, last_span.end))
}

/// Return whether one type value needs the `type` keyword to stay in type space.
fn type_value_needs_keyword(tree: &Tree, value: LocalNodeId<TypeExpression>) -> bool {
    match tree.get(value) {
        // enter type space through the prefix keyword
        TypeExpression::Readonly { .. } | TypeExpression::KeyOf { .. } => false,
        // relation operators promote their bare head into type space
        TypeExpression::Conditional { .. }
        | TypeExpression::Extends { .. }
        | TypeExpression::Implements { .. } => false,

        // infix types keep the space their head element claims
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            match elements.first() {
                Some(first) => type_value_needs_keyword(tree, *first),
                None => true,
            }
        }

        // preserve type space conservatively for every other head
        _ => true,
    }
}

/// Return whether one primary expression serializes empty infix annotations as postfix only.
pub(crate) fn primary_expression_uses_postfix_only_annotations(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    context.has_infix_annotation(expression_id)
        && (matches!(
            expression,
            Expression::ObjectExpression { properties, .. }
            | Expression::StructExpression { properties, .. } if properties.is_empty()
        ) || matches!(
            expression,
            Expression::ArrayExpression { elements } if elements.is_empty()
        ))
}

/// Write trailing annotations for one primary expression.
pub(crate) fn write_primary_expression_trailing_annotations<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    // empty literal infix annotations collapse to postfix-only spacing
    if primary_expression_uses_postfix_only_annotations(f.context(), expression_id, expression) {
        write!(f, [postfix_annotations(f.context(), expression_id)])?;
        return Ok(());
    }

    write!(
        f,
        [infix_or_postfix_annotations(f.context(), expression_id)]
    )
}

/// Format an array literal primary expression.
pub(crate) fn format_primary_array_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    elements_ids: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if elements_ids.is_empty() {
        let span = f.context().span(node_id);
        let has_dangling_comments = {
            let comments = f.context().comments();
            !comments.comments_before(span.end).is_empty()
        };

        if f.context().has_infix_annotation(node_id) {
            write!(
                f,
                [group(&format_args![
                    token("["),
                    block_indent(&format_with(|f| {
                        write!(f, [block_infix_annotations(f.context(), node_id)])?;
                        write!(f, [format_dangling_comments(span)])
                    })),
                    hard_line_break(),
                    token("]")
                ])]
            )?;
        } else if has_dangling_comments {
            write!(
                f,
                [group(&format_args![
                    token("["),
                    format_dangling_comments(span).with_block_indent(),
                    token("]")
                ])]
            )?;
        } else {
            write!(f, [token("[]")])?;
        }

        return Ok(());
    }

    let span = f.context().span(node_id);
    let has_newline_in_source = f.context().has_newline(span);
    let has_annotations = f.context().has_infix_annotation(node_id)
        || arguments_have_annotations(f.context(), elements_ids);

    let mut elements_are_inline_in_source = true;
    if has_annotations || (has_newline_in_source && elements_ids.len() > 1) {
        elements_are_inline_in_source = argument_range_is_inline(f.context(), elements_ids);
    }

    let mut should_expand_for_annotations = false;
    let mut can_keep_inline_outer_comment_array = false;

    // annotation sensitive expansion checks
    if has_annotations {
        let has_line_comments = elements_ids.iter().copied().any(|element_id| {
            let element_span = f.context().span(element_id);

            f.context()
                .source_comments_in_range(element_span.start, element_span.end)
                .iter()
                .copied()
                .any(|comment| comment.is_line())
        });

        can_keep_inline_outer_comment_array = elements_are_inline_in_source
            && array_elements_are_fill_candidates(f.context().tree, elements_ids)
            && array_has_only_outer_comments(f.context(), span, elements_ids);
        should_expand_for_annotations = has_line_comments || !elements_are_inline_in_source;
    }

    let tree = f.context().tree;
    let has_single_non_trivial_multiline_element = elements_ids.len() == 1
        && has_newline_in_source
        && !is_trivial_argument(tree, tree.get(elements_ids[0]));
    let has_multiline_non_inline_multi_element =
        has_newline_in_source && elements_ids.len() > 1 && !elements_are_inline_in_source;
    let should_break_nested_shape = array_expression_should_break(tree, elements_ids);

    let should_expand = (should_expand_for_annotations && !can_keep_inline_outer_comment_array)
        || has_single_non_trivial_multiline_element
        || has_multiline_non_inline_multi_element
        || should_break_nested_shape;
    let should_use_fill_layout = array_elements_are_fill_candidates(tree, elements_ids);

    if can_keep_inline_outer_comment_array {
        format_outer_comment_array(f, elements_ids)?;
    } else if should_use_fill_layout {
        format_fill_array(f, elements_ids)?;
    } else {
        // closing elisions need their comma in every layout
        let trailing_separator = if elements_ids
            .last()
            .is_some_and(|element_id| matches!(tree.get(*element_id), Argument::Elision))
        {
            TrailingSeparator::Mandatory
        } else {
            match f.context().options.trailing_comma {
                TrailingComma::None => TrailingSeparator::Omit,
                TrailingComma::Es5 | TrailingComma::All => TrailingSeparator::Allowed,
            }
        };
        write!(
            f,
            [group(&format_args![
                token("["),
                soft_block_indent(&separated_entries(
                    ",",
                    elements_ids,
                    trailing_separator,
                    None,
                )),
                token("]")
            ])
            .should_expand(should_expand)]
        )?;
    }

    Ok(())
}

/// Return the source semicolon separating one fixed array value and length.
fn fixed_array_source_separator(
    context: &TsppFormatContext<'_>,
    value: LocalNodeId<Expression>,
    length: LocalNodeId<Expression>,
) -> Option<TokenSpan> {
    let value_span = context.span(value);
    let length_start = context.expression_token_start(length);
    let separator = context.next_token_after_span(value_span)?;

    if separator.token.ty() != TokenType::Semicolon || separator.span.end > length_start {
        return None;
    }

    Some(separator)
}

/// Format a fixed array repeat literal primary expression.
fn format_primary_fixed_array_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    value: LocalNodeId<Expression>,
    length: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let body = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        let context = f.context();
        let value_span = context.span(value);
        let value_anchor_end = context
            .last_token_in_span(value_span)
            .map_or(value_span.end, |token| token.span.end);
        let length_start = context.expression_token_start(length);
        let separator = fixed_array_source_separator(context, value, length);
        let comments = context
            .comments()
            .comments_in_range(value_anchor_end, length_start);

        let (comments_before_separator, comments_after_separator) =
            if let Some(separator) = separator {
                let index = comments
                    .iter()
                    .position(|comment| comment.span.start >= separator.span.end)
                    .unwrap_or(comments.len());
                comments.split_at(index)
            } else {
                (comments, &[][..])
            };

        write!(f, [FormatNodeWithoutTrailingComments(value)])?;
        write_comment_slice(f, comments_before_separator)?;
        write!(f, [token(";")])?;
        write!(
            f,
            [
                FormatTrailingComments::Comments(comments_after_separator),
                soft_line_break_or_space(),
                length
            ]
        )
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

/// Return whether an array should expand under nested array and object rules.
fn array_expression_should_break(tree: &Tree, elements: &[LocalNodeId<Argument>]) -> bool {
    if elements.len() < 2 {
        return false;
    }

    let mut saw_array = false;
    let mut saw_object = false;

    for element_id in elements {
        let Argument::Positional { value, .. } = tree.get(*element_id) else {
            return false;
        };

        match tree.get(*value) {
            Expression::ArrayExpression { elements } => {
                if elements.len() < 2 || saw_object {
                    return false;
                }

                saw_array = true;
            }
            Expression::ObjectExpression { properties } => {
                if properties.len() < 2 || saw_array {
                    return false;
                }

                saw_object = true;
            }
            _ => return false,
        }
    }

    true
}

/// Format a tuple literal primary expression.
pub(crate) fn format_primary_tuple_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    elements_ids: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if elements_ids.is_empty() {
        write!(f, [token("()")])?;
    } else {
        // tuple layout
        let span = f.context().span(node_id);

        // expansion triggers
        let has_annotations = f.context().has_infix_annotation(node_id)
            || arguments_have_annotations(f.context(), elements_ids);
        let should_expand =
            has_annotations || (f.context().has_newline(span) && elements_ids.len() > 1);

        let trailing_separator = if elements_ids.len() == 1 {
            TrailingSeparator::Mandatory
        } else if f.context().options.trailing_comma == TrailingComma::None {
            TrailingSeparator::Omit
        } else {
            TrailingSeparator::Allowed
        };

        // tuple delimiters
        write!(
            f,
            [group(&format_args![
                token("("),
                soft_block_indent(&separated_entries(
                    ",",
                    elements_ids,
                    trailing_separator,
                    None,
                )),
                token(")")
            ])
            .should_expand(should_expand)]
        )?;
    }

    Ok(())
}

/// Format primary expression variants.
pub(crate) fn format_primary_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    let tree = f.context().tree;

    match expression {
        // identifier
        Expression::Identifier { name } => {
            write!(f, [*name])?;
        }

        // import meta
        Expression::ImportMeta => {
            write!(f, [Keyword::Import, token("."), token("meta")])?;
        }

        // import source
        Expression::ImportSource => {
            write!(f, [Keyword::Import, token("."), token("source")])?;
        }

        // this
        Expression::This => {
            write!(f, [Keyword::This])?;
        }

        // super
        Expression::Super => {
            write!(f, [Keyword::Super])?;
        }

        // scalar literal
        Expression::Literal(node) => {
            format_scalar_literal(node, tree.get_span(node_id), f)?;
        }

        // template literal
        Expression::TemplateExpression { value } => {
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // tagged template literal
        Expression::TaggedTemplateExpression {
            tag,
            generic_arguments,
            value,
        } => {
            write!(f, [*tag, block_infix_annotations(f.context(), node_id)])?;
            if !generic_arguments.is_empty() {
                format_generic_argument_list(f, generic_arguments)?;
            }
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // type expression
        Expression::Type { value } => {
            // restore type space for ambiguous heads
            if type_value_needs_keyword(tree, *value) {
                write!(f, [Keyword::Type, space(), value])?;
            } else {
                write!(f, [value])?;
            }
        }

        // inference hole
        Expression::Infer { .. } => {
            write!(f, [text("_")])?;
        }

        // array literal
        Expression::ArrayExpression {
            elements: elements_ids,
        } => {
            format_primary_array_expression(f, node_id, elements_ids)?;
        }

        // fixed array literal
        Expression::FixedArrayExpression { value, length } => {
            format_primary_fixed_array_expression(f, *value, *length)?;
        }

        // tuple literal
        Expression::TupleExpression {
            elements: elements_ids,
        } => {
            format_primary_tuple_expression(f, node_id, elements_ids)?;
        }

        // object literal
        Expression::ObjectExpression { properties } => {
            format_struct_literal(f, node_id, None, properties)?;
        }

        // struct literal
        Expression::StructExpression { ty, properties } => {
            format_struct_literal(f, node_id, Some(*ty), properties)?;
        }

        // tree literal
        Expression::TreeExpression {
            left,
            generic_arguments,
            attributes,
            children,
        } => {
            format_tree_literal_expression(
                f,
                node_id,
                left,
                generic_arguments,
                attributes,
                children,
            )?;
        }

        _ => return Ok(false),
    }

    Ok(true)
}
