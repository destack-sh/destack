use super::binary::format_binary_operand_with_grouping_parentheses;
use super::r#type::expression_has_type_grouping_semantics;
use super::types::{is_in_type_template_literal_interpolation, is_object_like_type_expression};
use crate::format::annotation::{format_raw_comment, prefix_annotations};
use crate::format::chain::transparent_inner_expression;
use crate::format::expression::{
    expression_has_leading_prefix_comment, should_drop_parenthesized_expression_wrapper,
    write_expression_without_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, BinaryOperator, Comment, Expression, LocalNodeId, NodeType, TokenType,
    TypeBinaryOperator, TypeLiteral,
};
use destack_fir::format::{Buffer, FormatResult, GroupId};
use destack_fir::prelude::{
    align, format_with, group, hard_line_break, if_group_breaks, if_group_fits_on_line, indent,
    soft_line_break_or_space, soft_line_indent_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;
use smallvec::SmallVec;

/// Return whether one expression has an own-line prefix annotation.
fn union_expression_has_own_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .annotation_ids(expression_id)
        .iter()
        .copied()
        .any(|annotation_id| {
            let annotation = context.annotation(annotation_id);
            if !matches!(
                annotation.position(),
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ) {
                return false;
            }

            context.annotation_starts_on_own_line(annotation_id)
                && !context.annotation_next_token_is_on_same_line(annotation_id)
        })
}

#[derive(Clone, Copy, Debug, Default)]
struct LeadingCommentsInfo {
    has_own_line_comment: bool,
    has_end_of_line_comment: bool,
    has_trailing_own_line_jsdoc_comment: bool,
}

/// Return the first non-trivia token start for one union owner or operand.
fn type_union_token_start(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    context.type_expression_token_start(expression_id)
}

/// Return whether one binary-like root has type grouping semantics.
fn binary_like_has_type_semantics(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    expression_has_type_grouping_semantics(context, node_id)
        || is_in_type_template_literal_interpolation(context, node_id)
}

/// Return whether one binary-like root should use type-union ownership.
pub(crate) fn binary_like_is_type_union(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> bool {
    if operator != BinaryOperator::ElementwiseOr {
        return false;
    }

    let normalized_root_id =
        transparent_type_binary_root_expression(context, node_id, BinaryOperator::ElementwiseOr);
    matches!(
        context.tree.get(normalized_root_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        }
    ) && binary_like_has_type_semantics(context, normalized_root_id)
}

/// Return whether one binary-like root should use type-intersection ownership.
pub(crate) fn binary_like_is_type_intersection(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> bool {
    if operator != BinaryOperator::ElementwiseAnd {
        return false;
    }

    let normalized_root_id =
        transparent_type_binary_root_expression(context, node_id, BinaryOperator::ElementwiseAnd);
    matches!(
        context.tree.get(normalized_root_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseAnd,
            ..
        }
    ) && binary_like_has_type_semantics(context, normalized_root_id)
}

/// Flatten operands for one binary-like owner according to its semantic family.
pub(crate) fn flatten_binary_like_operands(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]> {
    if binary_like_is_type_union(context, node_id, operator) {
        let normalized_root_id = transparent_type_binary_root_expression(
            context,
            node_id,
            BinaryOperator::ElementwiseOr,
        );
        return flatten_type_binary_expression(context, normalized_root_id, operator);
    }

    if binary_like_is_type_intersection(context, node_id, operator) {
        return flatten_type_binary_expression(context, node_id, operator);
    }

    super::binary::flatten_binary_expression(context, node_id, operator)
}

/// Flattens associative type binary chains while unwrapping redundant parentheses.
pub(crate) fn flatten_type_binary_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]> {
    let mut operands = SmallVec::new();
    flatten_type_binary_recursive(context, expression_id, target_operator, &mut operands, None);
    operands
}

/// Recursively flatten type binary chains and preserve operand operators.
fn flatten_type_binary_recursive(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    preceding_operator: Option<BinaryOperator>,
) {
    let expression_id =
        normalize_type_binary_operand_expression(context, expression_id, target_operator);

    if let Expression::Binary {
        left,
        operator,
        right,
    } = context.tree.get(expression_id)
        && *operator == target_operator
    {
        flatten_type_binary_recursive(
            context,
            *left,
            target_operator,
            operands,
            preceding_operator,
        );
        flatten_type_binary_recursive(context, *right, target_operator, operands, Some(*operator));
        return;
    }

    operands.push((preceding_operator, expression_id));
}

/// Remove redundant parenthesized wrappers around associative type operands.
fn normalize_type_binary_operand_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> LocalNodeId<Expression> {
    let mut current_id = expression_id;

    loop {
        let Expression::Parenthesized { expression } = context.tree.get(current_id) else {
            break;
        };

        if context.has_annotation(current_id) {
            break;
        }

        let inner_id = *expression;
        let inner_is_flattenable = matches!(
            context.tree.get(inner_id),
            Expression::Binary { operator, .. } if *operator == target_operator
        );
        let inner_is_parenthesized =
            matches!(context.tree.get(inner_id), Expression::Parenthesized { .. });
        if !inner_is_flattenable && !inner_is_parenthesized {
            break;
        }

        current_id = inner_id;
    }

    current_id
}

/// Return whether one type binary operand needs grouping parentheses.
pub(crate) fn type_binary_operand_needs_grouping_parentheses(
    context: &DestackFormatContext<'_>,
    parent_operator: BinaryOperator,
    operand_id: LocalNodeId<Expression>,
) -> bool {
    if !matches!(
        parent_operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    ) {
        return false;
    }

    if matches!(
        context.tree.get(operand_id),
        Expression::Parenthesized { .. }
    ) {
        return false;
    }

    let inner_id = transparent_inner_expression(context, operand_id);
    let Expression::Binary {
        operator: inner_operator,
        ..
    } = context.tree.get(inner_id)
    else {
        return false;
    };

    parent_operator == BinaryOperator::ElementwiseAnd
        && *inner_operator == BinaryOperator::ElementwiseOr
        && expression_has_type_grouping_semantics(context, inner_id)
}

/// Return the transparent inner expression for one type-binary chain root.
fn transparent_type_binary_chain_inner_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> Option<LocalNodeId<Expression>> {
    match context.tree.get(node_id) {
        Expression::Parenthesized {
            expression: inner_expression_id,
        } => Some(*inner_expression_id),
        Expression::Binary {
            operator: expression_operator,
            ..
        } if *expression_operator == operator
            && expression_has_type_grouping_semantics(context, node_id) =>
        {
            let operands = flatten_type_binary_expression(context, node_id, operator);
            if operands.len() == 1 && operands[0].1 != node_id {
                Some(operands[0].1)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Collect transparent owners from one type-binary root down to its normalized inner node.
pub(crate) fn collect_transparent_type_binary_root_owner_ids(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> SmallVec<[LocalNodeId<Expression>; 8]> {
    let mut owner_ids = SmallVec::new();
    let mut current_expression_id = node_id;
    owner_ids.push(node_id);

    while let Some(inner_expression_id) =
        transparent_type_binary_chain_inner_expression(context, current_expression_id, operator)
    {
        owner_ids.push(inner_expression_id);
        current_expression_id = inner_expression_id;
    }

    owner_ids
}

/// Return the normalized inner root for one type-binary chain.
pub(crate) fn transparent_type_binary_root_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> LocalNodeId<Expression> {
    collect_transparent_type_binary_root_owner_ids(context, node_id, operator)
        .last()
        .copied()
        .unwrap_or(node_id)
}

/// Return whether one operator expression owns its prefix annotations locally.
pub(crate) fn operator_expression_owns_prefix_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
    is_ignored: bool,
) -> bool {
    let expression_is_type_union = matches!(
        expression,
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        } if binary_like_is_type_union(context, expression_id, BinaryOperator::ElementwiseOr)
    );
    let parenthesized_delegates_union_prefix_annotations = if !is_ignored {
        match expression {
            Expression::Parenthesized {
                expression: inner_expression_id,
            } if should_drop_parenthesized_expression_wrapper(
                context,
                expression_id,
                *inner_expression_id,
            ) =>
            {
                let normalized_inner_id =
                    transparent_inner_expression(context, *inner_expression_id);
                let root_owns_prefix_annotation = {
                    let owner_ids = collect_transparent_type_binary_root_owner_ids(
                        context,
                        expression_id,
                        BinaryOperator::ElementwiseOr,
                    );
                    let union_root_id = owner_ids.last().copied().unwrap_or(expression_id);
                    owner_ids
                        .iter()
                        .copied()
                        .any(|owner_id| context.has_prefix_annotation(owner_id))
                        && (context.is_in_type_expression_root(expression_id)
                            || is_in_type_template_literal_interpolation(context, expression_id))
                        && {
                            let operands = flatten_type_binary_expression(
                                context,
                                union_root_id,
                                BinaryOperator::ElementwiseOr,
                            );
                            let first_operand_id =
                                operands.first().map_or(union_root_id, |operand| operand.1);
                            !context.has_prefix_annotation(first_operand_id)
                        }
                };

                matches!(
                    context.tree.get(normalized_inner_id),
                    Expression::Binary {
                        operator: BinaryOperator::ElementwiseOr,
                        ..
                    }
                ) && root_owns_prefix_annotation
            }
            _ => false,
        }
    } else {
        false
    };

    (!is_ignored && expression_is_type_union) || parenthesized_delegates_union_prefix_annotations
}

impl LeadingCommentsInfo {
    fn from_comment_nodes(context: &DestackFormatContext<'_>, comments: &[Comment]) -> Self {
        let mut info = Self::default();

        for comment in comments.iter().copied() {
            let comment_span = comment.span;
            let is_trailing_own_line_comment =
                context.span_has_newline_before_next_non_whitespace_token(comment_span);

            info.has_own_line_comment |= context.span_starts_on_own_line(comment_span);
            info.has_end_of_line_comment |=
                context.span_has_newline_before_next_non_whitespace_token(comment_span);
            info.has_trailing_own_line_jsdoc_comment |=
                is_trailing_own_line_comment && context.comment_is_doc(comment);
        }

        info
    }
}

/// Write raw leading comment nodes that belong before one union shell.
fn write_union_leading_comment_nodes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    for (index, comment) in comments.iter().copied().enumerate() {
        format_raw_comment(f, comment)?;

        if let Some(next_comment) = comments.get(index + 1).copied() {
            let next_comment_span = next_comment.span;

            let next_comment_starts_on_own_line =
                f.context().span_starts_on_own_line(next_comment_span);
            if comment.is_line() || next_comment_starts_on_own_line {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [space()])?;
            }
        }
    }

    Ok(())
}

/// Collect comment nodes before one union head and before any explicit leading `|`.
fn type_union_leading_comment_nodes(
    context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> Vec<Comment> {
    let Some(first_operand) = operands.first().map(|operand| operand.1) else {
        return Vec::new();
    };

    let leading_end = type_union_operand_separator_token_span(context, first_operand).map_or_else(
        || type_union_token_start(context, first_operand),
        |span| span.start,
    );
    let mut comments = context.comments().comments_before(leading_end).to_vec();

    comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
    comments.dedup_by_key(|comment| (comment.span.start, comment.span.end));
    comments
}

/// Return raw comments between one union separator and its operand.
fn type_union_comments_after_separator(
    context: &DestackFormatContext<'_>,
    operand_expression_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let Some(separator_span) =
        type_union_operand_separator_token_span(context, operand_expression_id)
    else {
        return Vec::new();
    };

    let operand_start = type_union_token_start(context, operand_expression_id);
    if separator_span.end >= operand_start {
        return Vec::new();
    }

    {
        let comments = context.comments();
        comments
            .comments_in_range(separator_span.end, operand_start)
            .to_vec()
    }
}

/// Write raw comments between one union separator and its operand.
fn write_union_comments_after_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operand_expression_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let comment_nodes = type_union_comments_after_separator(f.context(), operand_expression_id);
    if comment_nodes.is_empty() {
        return Ok(false);
    }

    if f.context().span_starts_on_own_line(comment_nodes[0].span) {
        write!(f, [hard_line_break()])?;
    }

    write!(
        f,
        [align(
            2,
            &format_with(|f| { write_union_leading_comment_nodes(f, &comment_nodes) })
        )]
    )?;

    let Some(last_comment) = comment_nodes.last().copied() else {
        return Ok(true);
    };

    if last_comment.is_line()
        || f.context()
            .span_has_newline_before_next_non_whitespace_token(last_comment.span)
    {
        write!(f, [hard_line_break(), space(), space()])?;
    } else {
        write!(f, [space()])?;
    }

    Ok(true)
}

/// Return whether one expression is object-like for union hug layout.
fn expression_is_hug_object_like_union_operand(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    is_object_like_type_expression(context, expression_id)
        || matches!(
            context.tree.get(expression_id),
            Expression::Identifier { .. }
                | Expression::QualifiedReference { .. }
                | Expression::Instantiation { .. }
        )
}

/// Return whether one expression is void-like for union hug layout.
fn expression_is_hug_void_like_union_operand(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    matches!(
        context.tree.get(expression_id),
        Expression::TypeLiteral(TypeLiteral::Void | TypeLiteral::Null)
    )
}

/// Return whether one type union should use object-and-void hug layout.
fn should_hug_type_union_operands(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> bool {
    let union_root_id =
        transparent_type_binary_root_expression(context, node_id, BinaryOperator::ElementwiseOr);

    if operands.len() <= 1 {
        return true;
    }

    if !operands
        .iter()
        .any(|operand| expression_is_hug_object_like_union_operand(context, operand.1))
    {
        return false;
    }

    let void_like_operand_count = operands
        .iter()
        .filter(|operand| expression_is_hug_void_like_union_operand(context, operand.1))
        .count();
    if operands.len().saturating_sub(1) != void_like_operand_count {
        return false;
    }

    let mut comment_check_start = context.span(union_root_id).start;
    for operand in operands.iter() {
        let operand_start = context.span(operand.1).start;
        if !context
            .comments_in_range(comment_check_start, operand_start)
            .is_empty()
        {
            return false;
        }

        comment_check_start = context.span(operand.1).end;
    }

    true
}

/// Return whether one union root ends with an own-line doc comment.
pub(crate) fn union_has_trailing_own_line_doc_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let union_root_id =
        transparent_type_binary_root_expression(context, node_id, BinaryOperator::ElementwiseOr);
    if !matches!(
        context.tree.get(union_root_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        }
    ) {
        return false;
    }

    let operands =
        flatten_type_binary_expression(context, union_root_id, BinaryOperator::ElementwiseOr);
    let leading_comment_nodes = type_union_leading_comment_nodes(context, union_root_id, &operands);
    let leading_comment_info =
        LeadingCommentsInfo::from_comment_nodes(context, &leading_comment_nodes);

    leading_comment_info.has_trailing_own_line_jsdoc_comment
}

/// Write one type-union operand after its separator.
fn write_type_union_operand_after_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operand_expression_id: LocalNodeId<Expression>,
    should_indent_union: bool,
    suppress_prefix_annotations: bool,
) -> FormatResult<()> {
    let suppress_operand_prefix = suppress_prefix_annotations;
    let operand_has_own_line_prefix_annotation = !suppress_operand_prefix
        && union_expression_has_own_line_prefix_annotation(f.context(), operand_expression_id);
    let format_operand = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        if operand_has_own_line_prefix_annotation && !suppress_prefix_annotations {
            write!(f, [operand_expression_id])
        } else if suppress_operand_prefix {
            write_expression_without_prefix_annotations(f, operand_expression_id)
        } else {
            format_binary_operand_with_grouping_parentheses(
                f,
                BinaryOperator::ElementwiseOr,
                operand_expression_id,
            )
        }
    });

    // multiline union operands align under `| `
    if f.context().options.indent_style.is_space() {
        write!(f, [align(2, &format_operand)])?;
    } else {
        if should_indent_union || operand_has_own_line_prefix_annotation {
            write!(f, [format_operand])?;
        } else {
            write!(f, [indent(&format_operand)])?;
        }
    }

    Ok(())
}

/// Render one type union in inline `A | B` form.
pub(crate) fn format_inline_type_union_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let Some(first_operand) = operands.first() else {
                return Ok(());
            };

            format_binary_operand_with_grouping_parentheses(
                f,
                BinaryOperator::ElementwiseOr,
                first_operand.1,
            )?;

            for operand in operands.iter().skip(1) {
                write!(
                    f,
                    [
                        space(),
                        token("|"),
                        space(),
                        format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            format_binary_operand_with_grouping_parentheses(
                                f,
                                BinaryOperator::ElementwiseOr,
                                operand.1,
                            )
                        }),
                    ]
                )?;
            }

            Ok(())
        }))]
    )
}

/// Render non-destack intersections with the standard object-like chain layout.
fn format_standard_intersection_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let last_index = operands.len().saturating_sub(1);
            let mut previous_is_object_like = false;
            let mut chain_is_indented = false;

            for (index, operand) in operands.iter().enumerate() {
                let current_is_object_like = is_object_like_type_expression(f.context(), operand.1);
                let current_has_own_line_prefix =
                    union_expression_has_own_line_prefix_annotation(f.context(), operand.1);
                let format_operand = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    format_binary_operand_with_grouping_parentheses(
                        f,
                        BinaryOperator::ElementwiseAnd,
                        operand.1,
                    )
                });

                // first operand
                if index == 0 {
                    write!(f, [format_operand])?;
                }
                // break when neither side is object-like, or when a leading prefix item owns the next line
                else if !(previous_is_object_like || current_is_object_like)
                    || current_has_own_line_prefix
                {
                    write!(f, [soft_line_indent_or_space(&format_operand)])?;
                }
                // otherwise keep the object-like chain behavior
                else {
                    write!(f, [space()])?;

                    // mixed object-like segments indent after the second transition
                    if !previous_is_object_like || !current_is_object_like {
                        chain_is_indented = index > 1;
                    }

                    if chain_is_indented {
                        write!(f, [indent(&format_operand)])?;
                    } else {
                        write!(f, [format_operand])?;
                    }
                }

                // separator
                if index < last_index {
                    write!(f, [space(), token("&")])?;
                }

                previous_is_object_like = current_is_object_like;
            }

            Ok(())
        }))]
    )
}

/// Format one type-intersection binary with the shared intersection layout.
pub(crate) fn format_type_intersection_binary_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> FormatResult<()> {
    format_standard_intersection_layout(f, operands)
}

/// Write the first operand in one multiline union layout.
fn write_multiline_type_union_first_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operand_expression_id: LocalNodeId<Expression>,
    union_group_id: Option<GroupId>,
    should_indent_union: bool,
    suppress_operand_prefix: bool,
    leading_comment_nodes: &[Comment],
) -> FormatResult<()> {
    let operand_has_explicit_separator =
        type_union_operand_separator_token_span(f.context(), operand_expression_id).is_some();

    if operand_has_explicit_separator {
        let leading_separator_stays_on_comment_line =
            leading_comment_nodes.last().is_some_and(|comment| {
                f.context().span_starts_on_own_line(comment.span)
                    && !f
                        .context()
                        .span_has_newline_before_next_non_whitespace_token(comment.span)
            });

        if leading_separator_stays_on_comment_line {
            write!(f, [token("|"), space()])?;
        } else {
            write!(
                f,
                [if_group_breaks(&format_args![
                    soft_line_break_or_space(),
                    token("|"),
                    space()
                ])
                .with_group_id(union_group_id)]
            )?;
        }
    }

    let wrote_separator_comments = write_union_comments_after_separator(f, operand_expression_id)?;

    write_type_union_operand_after_separator(
        f,
        operand_expression_id,
        should_indent_union,
        suppress_operand_prefix || wrote_separator_comments,
    )
}

/// Write one following operand in one multiline union layout.
fn write_multiline_type_union_following_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    previous_expression_id: LocalNodeId<Expression>,
    operand_expression_id: LocalNodeId<Expression>,
    union_group_id: Option<GroupId>,
    should_indent_union: bool,
) -> FormatResult<()> {
    let previous_end_of_line_comments = f
        .context()
        .end_of_line_raw_comments_after(f.context().span(previous_expression_id).end);
    let between_separator_comment_nodes = if let Some(separator_span) =
        type_union_operand_separator_token_span(f.context(), operand_expression_id)
    {
        let previous_operand_span = f.context().span(previous_expression_id);

        if previous_operand_span.file == separator_span.file
            && previous_operand_span.end < separator_span.start
        {
            {
                let comments = f.context().comments();
                comments
                    .comments_in_range(previous_operand_span.end, separator_span.start)
                    .to_vec()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    if !between_separator_comment_nodes.is_empty() {
        let first_comment_span = between_separator_comment_nodes[0].span;
        if f.context().span_starts_on_own_line(first_comment_span) {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }

        write_union_leading_comment_nodes(f, &between_separator_comment_nodes)?;

        if let Some(separator_span) =
            type_union_operand_separator_token_span(f.context(), operand_expression_id)
        {
            let Some(last_comment) = between_separator_comment_nodes.last().copied() else {
                return write_type_union_operand_after_separator(
                    f,
                    operand_expression_id,
                    should_indent_union,
                    false,
                );
            };
            let last_comment_span = last_comment.span;
            let gap_span = Span::new(
                last_comment_span.file,
                last_comment_span.end,
                separator_span.start,
            );
            if f.context().has_newline(gap_span) {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [space()])?;
            }
        }
    } else if !previous_end_of_line_comments.is_empty() {
        write!(f, [hard_line_break()])?;
    } else {
        write!(
            f,
            [
                if_group_breaks(&soft_line_break_or_space()).with_group_id(union_group_id),
                if_group_fits_on_line(&space()).with_group_id(union_group_id)
            ]
        )?;
    }

    write!(f, [token("|"), space()])?;

    let suppress_operand_prefix = write_union_comments_after_separator(f, operand_expression_id)?;
    write_type_union_operand_after_separator(
        f,
        operand_expression_id,
        should_indent_union,
        suppress_operand_prefix,
    )
}

/// Write the multiline union body after the leading shell.
fn write_multiline_type_union_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    union_group_id: Option<GroupId>,
    should_indent_union: bool,
    suppress_first_operand_prefix: bool,
    leading_comment_nodes: &[Comment],
) -> FormatResult<()> {
    let Some(first_operand) = operands.first() else {
        return Ok(());
    };

    write_multiline_type_union_first_operand(
        f,
        first_operand.1,
        union_group_id,
        should_indent_union,
        suppress_first_operand_prefix,
        leading_comment_nodes,
    )?;

    for (previous_operand, operand) in operands.iter().zip(operands.iter().skip(1)) {
        write_multiline_type_union_following_operand(
            f,
            previous_operand.1,
            operand.1,
            union_group_id,
            should_indent_union,
        )?;
    }

    Ok(())
}

/// Write the leading shell around one multiline union layout.
fn write_multiline_type_union_content<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    root_owns_prefix_annotation: bool,
    leading_comment_nodes: &[Comment],
    leading_comment_info: LeadingCommentsInfo,
    union_group: impl destack_fir::format::Format<DestackFormatContext<'ast>>,
) -> FormatResult<()> {
    let should_break_before_leading_comments = leading_comment_nodes
        .last()
        .is_some_and(|comment| f.context().span_starts_on_own_line(comment.span));

    if should_break_before_leading_comments {
        write!(f, [hard_line_break()])?;
    }

    if !leading_comment_nodes.is_empty() {
        write_union_leading_comment_nodes(f, leading_comment_nodes)?;

        if let Some(last_comment) = leading_comment_nodes.last().copied() {
            let last_comment_span = last_comment.span;
            if last_comment.is_line() || leading_comment_info.has_trailing_own_line_jsdoc_comment {
                write!(f, [hard_line_break()])?;
            } else if f
                .context()
                .span_has_newline_before_next_non_whitespace_token(last_comment_span)
            {
                write!(f, [soft_line_break_or_space()])?;
            } else {
                write!(f, [space()])?;
            }
        }
    }

    if root_owns_prefix_annotation && leading_comment_nodes.is_empty() {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    write!(f, [union_group])
}

/// Format one multiline type union layout.
fn format_multiline_type_union_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    root_owns_prefix_annotation: bool,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
    leading_comment_nodes: &[Comment],
    leading_comment_info: LeadingCommentsInfo,
    should_force_expand: bool,
) -> FormatResult<()> {
    let union_group_id = f.group_id("type_union");

    // union top
    let mut union_top_id = node_id;
    let mut current_expression_id = node_id;
    while let Some((parent_id, parent_type)) = f.context().parent(current_expression_id) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        match f.context().tree.get(parent_expression_id) {
            Expression::Parenthesized {
                expression: inner_expression_id,
            } if *inner_expression_id == current_expression_id => {
                union_top_id = parent_expression_id;
                current_expression_id = parent_expression_id;
            }
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr,
                ..
            } if expression_has_type_grouping_semantics(f.context(), parent_expression_id) => {
                let operands = flatten_type_binary_expression(
                    f.context(),
                    parent_expression_id,
                    BinaryOperator::ElementwiseOr,
                );
                if operands.len() != 1 || operands[0].1 != current_expression_id {
                    break;
                }

                union_top_id = parent_expression_id;
                current_expression_id = parent_expression_id;
            }
            _ => break,
        }
    }

    // indent ownership
    let should_indent_union = if leading_comment_info.has_trailing_own_line_jsdoc_comment {
        false
    } else if is_in_type_template_literal_interpolation(f.context(), union_top_id) {
        true
    } else if f.context().is_in_assignment_like_type_root(union_top_id) {
        true
    } else if let Some((parent_id, parent_type)) = f.context().parent(union_top_id) {
        if parent_type == NodeType::Argument {
            false
        } else if parent_type == NodeType::Expression {
            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            !matches!(
                f.context().tree.get(parent_expression_id),
                Expression::TypeBinary {
                    operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
                    ..
                }
            )
        } else {
            true
        }
    } else {
        true
    };

    // grouped body
    let suppress_first_operand_prefix = operands.first().is_some_and(|operand| {
        let operand_token_start = type_union_token_start(f.context(), operand.1);
        !type_union_comments_after_separator(f.context(), operand.1).is_empty()
            || leading_comment_nodes
                .iter()
                .copied()
                .any(|comment| comment.attached_to == operand_token_start)
    });
    let union_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_multiline_type_union_body(
            f,
            operands,
            Some(union_group_id),
            should_indent_union,
            suppress_first_operand_prefix,
            leading_comment_nodes,
        )
    });
    let union_group = group(&union_body)
        .with_id(Some(union_group_id))
        .should_expand(should_force_expand);
    let union_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_multiline_type_union_content(
            f,
            node_id,
            root_owns_prefix_annotation,
            leading_comment_nodes,
            leading_comment_info,
            union_group.clone(),
        )
    });

    if should_indent_union {
        write!(f, [group(&indent(&union_content))])
    } else {
        write!(f, [group(&union_content)])
    }
}

/// Return the `|` token that structurally owns one type-union operand.
pub(crate) fn type_union_operand_separator_token_span(
    context: &DestackFormatContext<'_>,
    operand_expression_id: LocalNodeId<Expression>,
) -> Option<destack_source::Span> {
    let operand_span = context.span(operand_expression_id);
    let operand_token_start = context
        .first_non_trivia_token_in_span(operand_span)
        .map_or(operand_span.start, |token| token.span.start);
    let mut token_index = context
        .tokens
        .partition_point(|token| token.span.end <= operand_token_start);

    while token_index > 0 {
        token_index -= 1;
        let token = context.tokens[token_index];

        match token.token.ty {
            TokenType::Whitespace
            | TokenType::Newline
            | TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment => continue,
            TokenType::ElementwiseOr => return Some(token.span),
            _ => return None,
        }
    }

    None
}

/// Format one type-union binary with inline or leading-pipe layout.
pub(crate) fn format_type_union_binary_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    operands: &SmallVec<[(Option<BinaryOperator>, LocalNodeId<Expression>); 8]>,
) -> FormatResult<()> {
    let leading_comment_nodes = type_union_leading_comment_nodes(f.context(), node_id, operands);
    let leading_comment_info =
        LeadingCommentsInfo::from_comment_nodes(f.context(), &leading_comment_nodes);

    // root-owned prefix annotations
    let root_owns_prefix_annotation = {
        let owner_ids = collect_transparent_type_binary_root_owner_ids(
            f.context(),
            node_id,
            BinaryOperator::ElementwiseOr,
        );
        let union_root_id = owner_ids.last().copied().unwrap_or(node_id);
        owner_ids
            .iter()
            .copied()
            .any(|owner_id| f.context().has_prefix_annotation(owner_id))
            && (f.context().is_in_type_expression_root(node_id)
                || is_in_type_template_literal_interpolation(f.context(), node_id))
            && {
                let first_operand_id = operands.first().map_or(union_root_id, |operand| operand.1);
                !f.context().has_prefix_annotation(first_operand_id)
            }
    };

    // layout selection
    let should_hug = should_hug_type_union_operands(f.context(), node_id, operands);
    let has_operand_prefix_comments = operands
        .iter()
        .any(|operand| expression_has_leading_prefix_comment(f.context(), operand.1));
    let has_breaking_separator_comments = operands.iter().any(|operand| {
        let separator_comments = if let Some(separator_span) =
            type_union_operand_separator_token_span(f.context(), operand.1)
        {
            let operand_span = f.context().span(operand.1);
            if separator_span.file == operand_span.file && separator_span.end < operand_span.start {
                {
                    let comments = f.context().comments();
                    comments
                        .comments_in_range(separator_span.end, operand_span.start)
                        .to_vec()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        !separator_comments.is_empty()
            && !separator_comments.iter().copied().all(|comment| {
                let comment_span = comment.span;

                comment.is_block()
                    && !f.context().has_newline(comment_span)
                    && !f.context().span_starts_on_own_line(comment_span)
                    && !f
                        .context()
                        .span_has_newline_before_next_non_whitespace_token(comment_span)
            })
    });
    let has_breaking_postfix_comments = operands.iter().enumerate().any(|(index, operand)| {
        let is_last_operand = index + 1 == operands.len();
        let has_postfix_comment = !f
            .context()
            .end_of_line_raw_comments_after(f.context().span(operand.1).end)
            .is_empty();
        has_postfix_comment && !is_last_operand
    });
    let has_breaking_operand_prefix_comments = has_operand_prefix_comments
        && operands.iter().enumerate().skip(1).any(|(index, operand)| {
            let is_last_operand = index + 1 == operands.len();
            expression_has_leading_prefix_comment(f.context(), operand.1) && !is_last_operand
        });
    let should_expand = has_breaking_operand_prefix_comments
        || has_breaking_postfix_comments
        || has_breaking_separator_comments;

    if should_hug {
        if root_owns_prefix_annotation {
            write!(f, [prefix_annotations(f.context(), node_id)])?;
        }
        return format_inline_type_union_layout(f, operands);
    }

    format_multiline_type_union_layout(
        f,
        node_id,
        root_owns_prefix_annotation,
        operands,
        &leading_comment_nodes,
        leading_comment_info,
        should_expand,
    )
}
