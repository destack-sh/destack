use super::object::{format_fill_array, format_outer_comment_array, format_struct_literal};
use super::parentheses::parenthesized_expression_needs_preserved_wrapper;
use super::{
    array_elements_are_fill_candidates, array_has_only_outer_comments, is_trivial_argument,
    sequence_expression_needs_parens,
};
use crate::annotation::{
    FormatTrailingComments, block_infix_annotations, format_dangling_comments,
    infix_or_postfix_annotations, postfix_annotations, write_comment_slice,
};
use crate::chain::transparent_inner_expression;
use crate::collection::literal::{format_scalar_literal, format_template_literal};
use crate::collection::{TrailingSeparator, separated_entries};
use crate::context::{FormatNodeWithoutTrailingComments, with_following_span_start};
use crate::declaration::expression_is_in_statement_context;
use crate::operator::format_generic_argument_list;
use crate::tree::format_tree_literal_expression;
use crate::{DestackFormatContext, DestackFormatter};
use destack_dir::{
    Argument, Expression, Keyword, LocalNodeId, NodeType, TokenSpan, TokenType, Tree,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, indent, line_suffix_boundary,
    soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::TrailingComma;

/// One preserved explicit parenthesized wrapper layout.
enum ParenthesizedExpressionLayout {
    /// Keep the wrapper inline.
    Inline,
    /// Keep the wrapper with an explicit multiline body.
    Multiline,
}

/// Return one explicit parenthesized wrapper layout when the wrapper must stay visible.
fn parenthesized_expression_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> Option<ParenthesizedExpressionLayout> {
    let outer_span = context.span(node_id);
    if !parenthesized_expression_needs_preserved_wrapper(context, node_id, expression_id) {
        return None;
    }

    if context.has_newline(outer_span) {
        return Some(ParenthesizedExpressionLayout::Multiline);
    }

    Some(ParenthesizedExpressionLayout::Inline)
}

/// Return whether any argument in one collection has annotations.
fn arguments_have_annotations(
    context: &DestackFormatContext<'_>,
    argument_ids: &[LocalNodeId<Argument>],
) -> bool {
    argument_ids
        .iter()
        .copied()
        .any(|argument_id| context.has_annotation(argument_id))
}

/// Return whether one argument collection is inline between its outer spans.
fn argument_range_is_inline(
    context: &DestackFormatContext<'_>,
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

/// Return whether a sequence expression tail should be indented.
fn sequence_expression_tail_needs_indent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return true;
    };

    if parent_type != NodeType::Expression {
        return expression_is_in_statement_context(context, node_id);
    }

    matches!(
        context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
        Expression::For { .. }
    )
}

/// Return the following sibling boundary for one sequence expression entry.
fn sequence_expression_entry_following_span_start(
    context: &DestackFormatContext<'_>,
    expressions: &[LocalNodeId<Expression>],
    index: usize,
) -> u32 {
    expressions
        .get(index + 1)
        .map(|expression_id| context.span(*expression_id).start)
        .unwrap_or_else(|| context.following_span_start())
}

/// Write one sequence expression entry with sibling trivia boundaries.
fn write_sequence_expression_entry<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expressions: &[LocalNodeId<Expression>],
    index: usize,
) -> FormatResult<()> {
    let expression_id = expressions[index];
    let following_span_start =
        sequence_expression_entry_following_span_start(f.context(), expressions, index);

    with_following_span_start(f, following_span_start, |f| write!(f, [expression_id]))
}

/// Return whether one primary expression serializes empty infix annotations as postfix only.
pub(crate) fn primary_expression_uses_postfix_only_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    context.has_infix_annotation(expression_id)
        && (matches!(
            expression,
            Expression::ObjectExpression { properties, .. } if properties.is_empty()
        ) || matches!(
            expression,
            Expression::ArrayExpression { elements } if elements.is_empty()
        ))
}

/// Write trailing annotations for one primary expression.
pub(crate) fn write_primary_expression_trailing_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
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
    f: &mut DestackFormatter<'ast, '_>,
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

    if array_has_sparse_holes(f.context(), elements_ids) {
        let span = f.context().span(node_id);
        let has_newline_in_source = f.context().has_newline(span);
        let has_sparse_annotations = f.context().has_infix_annotation(node_id)
            || arguments_have_annotations(f.context(), elements_ids);

        if has_newline_in_source || has_sparse_annotations {
            let trailing_separator = match f.context().options.trailing_comma {
                TrailingComma::None => TrailingSeparator::Omit,
                TrailingComma::Es5 | TrailingComma::All => TrailingSeparator::Allowed,
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
                .should_expand(has_newline_in_source)]
            )?;
        } else {
            format_sparse_array_literal(f, elements_ids)?;
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
                .comment_tokens_in_range(element_span.start, element_span.end)
                .iter()
                .copied()
                .any(|comment| f.context().comment_is_line(comment))
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
        let trailing_separator = match f.context().options.trailing_comma {
            TrailingComma::None => TrailingSeparator::Omit,
            TrailingComma::Es5 | TrailingComma::All => TrailingSeparator::Allowed,
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
    context: &DestackFormatContext<'_>,
    value: LocalNodeId<Expression>,
    length: LocalNodeId<Expression>,
) -> Option<TokenSpan> {
    let value_span = context.span(value);
    let length_start = context.expression_token_start(length);
    let separator = context.next_non_trivia_token_after_span(value_span)?;

    if separator.token.ty != TokenType::Semicolon || separator.span.end > length_start {
        return None;
    }

    Some(separator)
}

/// Format a fixed array repeat literal primary expression.
fn format_primary_fixed_array_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value: LocalNodeId<Expression>,
    length: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let context = f.context();
        let value_span = context.span(value);
        let value_anchor_end = context
            .last_non_trivia_token_in_span(value_span)
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
            Expression::ObjectExpression {
                ty: None,
                properties,
            } => {
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
    f: &mut DestackFormatter<'ast, '_>,
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

/// Return whether an array literal contains sparse elision slots.
fn array_has_sparse_holes(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    elements
        .iter()
        .copied()
        .any(|argument_id| argument_is_sparse_hole(context, argument_id))
}

/// Return whether an array element argument is a sparse hole.
fn argument_is_sparse_hole(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Argument::Positional { value, .. } = context.tree.get(argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, *value);
    matches!(context.tree.get(value_id), Expression::Stub)
}

/// Format sparse arrays while preserving elision comma count.
fn format_sparse_array_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [token("[")])?;

    let mut expects_element = true;
    for (index, argument_id) in elements.iter().copied().enumerate() {
        let is_hole = argument_is_sparse_hole(f.context(), argument_id);

        if !expects_element {
            write!(f, [token(",")])?;
            if !is_hole {
                write!(f, [space()])?;
            }
            expects_element = true;
        }

        if is_hole {
            write!(f, [token(",")])?;
            let next_is_non_hole = elements
                .get(index + 1)
                .is_some_and(|next_id| !argument_is_sparse_hole(f.context(), *next_id));
            if next_is_non_hole {
                write!(f, [space()])?;
            }
            continue;
        }

        write!(f, [argument_id])?;
        expects_element = false;
    }

    write!(f, [token("]")])?;
    Ok(())
}

/// Format primary expression variants.
pub(crate) fn format_primary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    let tree = f.context().tree;

    match expression {
        // identifier
        Expression::Identifier { name } => {
            write!(f, [*name])?;
        }

        // qualified reference
        Expression::QualifiedReference {
            path,
            generic_arguments,
        } => {
            let _ = node_id;

            write!(f, [path])?;

            if !generic_arguments.is_empty() {
                format_generic_argument_list(f, generic_arguments)?;
            }
        }

        // private identifier
        Expression::PrivateIdentifier { name } => {
            write!(f, [token("#"), *name])?;
        }

        // import meta
        Expression::ImportMeta => {
            write!(f, [Keyword::Import, token("."), token("meta")])?;
        }

        // new target
        Expression::NewTarget => {
            write!(f, [Keyword::New, token("."), token("target")])?;
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
        Expression::ScalarLiteral(node) => {
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
            write!(f, [value])?;
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

        // sequence expression (JS/TS comma operator)
        Expression::SequenceExpression { expressions } => {
            if expressions.is_empty() {
                write!(f, [token("()")])?;
            } else {
                let format_sequence = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    let joiner_separator = format_with(|f| {
                        write!(
                            f,
                            [
                                token(","),
                                line_suffix_boundary(),
                                soft_line_break_or_space()
                            ]
                        )
                    });
                    let rest_expressions = expressions
                        .get(1..)
                        .expect("non-empty sequence expression has rest slice");

                    write_sequence_expression_entry(f, expressions, 0)?;

                    if !rest_expressions.is_empty() {
                        write!(f, [token(","), line_suffix_boundary()])?;
                    }

                    let rest = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(f, [soft_line_break_or_space()])?;

                        for index in 1..expressions.len() {
                            if index > 1 {
                                write!(f, [joiner_separator])?;
                            }

                            write_sequence_expression_entry(f, expressions, index)?;
                        }

                        Ok(())
                    });

                    if sequence_expression_tail_needs_indent(f.context(), node_id) {
                        write!(f, [indent(&rest)])
                    } else {
                        write!(f, [rest])
                    }
                });

                if sequence_expression_needs_parens(f.context(), node_id) {
                    write!(
                        f,
                        [group(&format_args![
                            token("("),
                            format_sequence,
                            token(")")
                        ])]
                    )?;
                } else {
                    write!(f, [group(&format_sequence)])?;
                }
            }
        }

        // struct literal
        Expression::ObjectExpression { ty, properties } => {
            format_struct_literal(f, node_id, ty, properties)?;
        }

        // tree literal
        Expression::TreeExpression {
            left,
            generic_arguments,
            arguments,
            elements,
        } => {
            format_tree_literal_expression(
                f,
                node_id,
                left,
                generic_arguments,
                arguments,
                elements,
            )?;
        }

        // grouped expressions keep their wrapper when comments or line breaks depend on it
        Expression::Parenthesized { expression } => {
            match parenthesized_expression_layout(f.context(), node_id, *expression) {
                Some(ParenthesizedExpressionLayout::Inline) => {
                    write!(f, [token("("), *expression, token(")")])?;
                }
                Some(ParenthesizedExpressionLayout::Multiline) => {
                    write!(
                        f,
                        [group(&format_args![
                            token("("),
                            block_indent(expression),
                            hard_line_break(),
                            token(")")
                        ])]
                    )?;
                }
                None => write!(f, [*expression])?,
            }
        }
        _ => return Ok(false),
    }

    Ok(true)
}
