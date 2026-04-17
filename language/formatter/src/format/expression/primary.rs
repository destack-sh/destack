use super::object::{format_boundary_comment_array, format_fill_array, format_struct_literal};
use super::parentheses::format_primary_parenthesized_expression;
use super::path::{format_path_expression, primary_expression_skips_boundary_annotations};
use super::{
    array_elements_are_fill_candidates, array_has_only_boundary_comments, is_trivial_argument,
    sequence_expression_needs_parens,
};
use crate::format::annotation::{
    block_infix_annotations, infix_or_postfix_annotations,
    infix_or_postfix_annotations_without_line_suffix_boundary, postfix_annotations,
};
use crate::format::chain::transparent_inner_expression;
use crate::format::collection::literal::{format_scalar_literal, format_template_literal};
use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::format::operator::expression_is_type_position;
use crate::format::tree::format_tree_literal_expression;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Argument, Expression, Keyword, LocalNodeId};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, line_suffix_boundary, soft_block_indent,
    soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

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

    // boundary path comments are rendered inside the path printer
    if primary_expression_skips_boundary_annotations(f.context(), expression_id, expression) {
        write!(
            f,
            [infix_or_postfix_annotations_without_line_suffix_boundary(
                f.context(),
                expression_id
            )]
        )?;
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
        if f.context().has_infix_annotation(node_id) {
            write!(
                f,
                [group(&format_args![
                    token("["),
                    block_indent(&block_infix_annotations(f.context(), node_id)),
                    hard_line_break(),
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
                destack_workspace::TrailingComma::None => TrailingSeparator::Omit,
                destack_workspace::TrailingComma::Es5 | destack_workspace::TrailingComma::All => {
                    TrailingSeparator::Allowed
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
    let mut can_keep_inline_boundary_comment_array = false;

    // annotation sensitive expansion checks
    if has_annotations {
        let has_line_comments = elements_ids.iter().copied().any(|element_id| {
            let element_span = f.context().span(element_id);

            f.context()
                .comments_in_range(element_span.start, element_span.end)
                .iter()
                .copied()
                .any(|comment| f.context().comment_is_line(comment))
        });

        can_keep_inline_boundary_comment_array = elements_are_inline_in_source
            && array_elements_are_fill_candidates(f.context().tree, elements_ids)
            && array_has_only_boundary_comments(f.context(), span, elements_ids);
        should_expand_for_annotations = has_line_comments || !elements_are_inline_in_source;
    }

    let tree = f.context().tree;
    let has_single_non_trivial_multiline_element = elements_ids.len() == 1
        && has_newline_in_source
        && !is_trivial_argument(tree, tree.get(elements_ids[0]));
    let has_multiline_non_inline_multi_element =
        has_newline_in_source && elements_ids.len() > 1 && !elements_are_inline_in_source;

    let should_expand = (should_expand_for_annotations && !can_keep_inline_boundary_comment_array)
        || has_single_non_trivial_multiline_element
        || has_multiline_non_inline_multi_element;
    let should_use_fill_layout =
        !has_annotations && array_elements_are_fill_candidates(tree, elements_ids);

    if can_keep_inline_boundary_comment_array {
        format_boundary_comment_array(f, elements_ids)?;
    } else if should_use_fill_layout {
        format_fill_array(f, elements_ids)?;
    } else {
        let trailing_separator = match f.context().options.trailing_comma {
            destack_workspace::TrailingComma::None => TrailingSeparator::Omit,
            destack_workspace::TrailingComma::Es5 | destack_workspace::TrailingComma::All => {
                TrailingSeparator::Allowed
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

        let trailing_separator = if expression_is_type_position(f.context(), node_id) {
            match f.context().options.trailing_comma {
                destack_workspace::TrailingComma::None => TrailingSeparator::Omit,
                destack_workspace::TrailingComma::Es5 | destack_workspace::TrailingComma::All => {
                    TrailingSeparator::Allowed
                }
            }
        } else {
            TrailingSeparator::Mandatory
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
            format_path_expression(f, node_id, path, generic_arguments)?;
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
        Expression::TaggedTemplateExpression { tag, value } => {
            write!(f, [tag, block_infix_annotations(f.context(), node_id)])?;
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // type shell
        Expression::Type { value } => {
            write!(f, [value])?;
        }

        // array literal
        Expression::ArrayExpression {
            elements: elements_ids,
        } => {
            format_primary_array_expression(f, node_id, elements_ids)?;
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
                let format_sequence = format_with(|f| {
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
                    let mut joiner = f.join_with(joiner_separator);
                    joiner.entries(expressions);
                    joiner.finish()
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
            arguments,
            elements,
        } => {
            format_tree_literal_expression(f, node_id, left, arguments, elements)?;
        }

        // parenthesized
        Expression::Parenthesized { expression } => {
            format_primary_parenthesized_expression(f, node_id, *expression)?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}
