use super::union::{
    binary_like_is_type_union, flatten_binary_like_operands, flatten_type_binary_expression,
    format_inline_type_union_layout, transparent_type_binary_root_expression,
};
use crate::format::annotation::{
    raw_prefix_comment_nodes, write_inline_prefix_annotations as write_annotation_prefix_sequence,
};
use crate::format::chain::{
    should_expand_static_argument_list, static_argument_list_is_hug_safe,
    transparent_inner_expression,
};
use crate::format::declaration::{
    parenthesized_wraps_decorated_class_extends_head,
    parenthesized_wraps_prefix_annotated_class_extends_head,
};
use crate::format::expression::{
    parenthesized_has_explicit_delimiters, write_expression_without_prefix_annotations,
};
use crate::format::operator::types::is_simple_type_binary_left_expression;
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, Argument, BinaryOperator, Comment, CommentStyle, Doc, DocStyle, Expression,
    LocalNodeId, Member, NodeType, Property, TokenType, TypeBinaryOperator, TypeUnaryOperator,
};
use destack_fir::format::{Buffer, FormatResult, RemoveSoftLinesBuffer};
use destack_fir::prelude::{
    format_with, group, hard_line_break, soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};

/// Return whether annotations are only type grouping prefix trivia for this expression.
pub(crate) fn expression_has_only_type_grouping_prefix_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let annotation_ids = context.annotation_ids(expression_id);
    if annotation_ids.is_empty() {
        return false;
    }

    annotation_ids.iter().copied().all(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let is_supported_prefix_annotation = matches!(
            annotation,
            Annotation::Doc {
                position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                ..
            }
        );
        if !is_supported_prefix_annotation {
            return false;
        }

        matches!(
            context.annotation_next_non_whitespace_token_type(annotation_id),
            Some(
                TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
                    | TokenType::ElementwiseOr
                    | TokenType::ElementwiseAnd
            )
        )
    })
}

/// Return the innermost transparent type grouping expression under parenthesized wrappers.
pub(crate) fn normalize_parenthesized_type_grouping_inner_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = expression_id;

    while let Expression::Parenthesized { expression } = context.tree.get(current_id) {
        if context.has_annotation(current_id)
            && !expression_has_only_type_grouping_prefix_annotations(context, current_id)
        {
            break;
        }

        current_id = *expression;
    }

    current_id
}

/// Return whether a binary operator is associative in type slot grouping.
pub(crate) fn is_associative_type_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    )
}

/// Collect raw source comments that sit directly before one type-position expression.
pub(crate) fn leading_raw_type_position_comment_nodes(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Comment>> {
    if !expression_is_type_position(context, expression_id) {
        return Vec::new();
    }

    let expression_span = context.span(expression_id);
    let first_token_type = context
        .first_non_trivia_token_in_span(expression_span)
        .map(|token| token.token.ty);
    if matches!(
        first_token_type,
        Some(TokenType::ElementwiseOr | TokenType::ElementwiseAnd)
    ) {
        return Vec::new();
    }

    let Some(previous_token) = context.previous_non_trivia_token_before_span(expression_span)
    else {
        return Vec::new();
    };
    let previous_owner_start = match previous_token.token.ty {
        TokenType::LineComment
        | TokenType::BlockComment
        | TokenType::DocLineComment
        | TokenType::DocBlockComment => previous_token.span.start,
        _ => previous_token.span.end,
    };
    if previous_token.span.file != expression_span.file
        || previous_owner_start >= expression_span.start
    {
        return Vec::new();
    }

    let mut comment_ids =
        context.comment_nodes_in_range(previous_owner_start, expression_span.start);
    let prefix_comment_ids = raw_prefix_comment_nodes(context, expression_id);
    comment_ids.retain(|comment_id| !prefix_comment_ids.contains(comment_id));

    comment_ids
}

/// Write raw source comments that precede one type-position expression.
pub(crate) fn write_leading_raw_type_position_comment_nodes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comment_ids: &[LocalNodeId<Comment>],
) -> FormatResult<()> {
    for (index, comment_id) in comment_ids.iter().copied().enumerate() {
        write!(f, [comment_id])?;

        if let Some(next_comment_id) = comment_ids.get(index + 1).copied() {
            let current_span = f.context().span(comment_id);
            let next_span = f.context().span(next_comment_id);
            let gap_span =
                destack_source::Span::new(current_span.file, current_span.end, next_span.start);

            if f.context().tree.get(comment_id).style == CommentStyle::Slash
                || f.context().has_newline(gap_span)
            {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [space()])?;
            }
        }
    }

    if let Some(last_comment_id) = comment_ids.last().copied() {
        let last_comment_span = f.context().span(last_comment_id);
        let comment_token_type = f
            .context()
            .first_non_trivia_token_in_span(last_comment_span)
            .map(|token| token.token.ty);
        let comment_requires_break = matches!(
            comment_token_type,
            Some(TokenType::LineComment | TokenType::DocLineComment | TokenType::DocBlockComment)
        );

        if f.context().tree.get(last_comment_id).style == CommentStyle::Slash
            || comment_requires_break
        {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Return whether one raw type-position comment must force a following break.
fn type_position_comment_requires_break_after(
    context: &DestackFormatContext<'_>,
    comment_id: LocalNodeId<Comment>,
) -> bool {
    if context.tree.get(comment_id).style == CommentStyle::Slash {
        return true;
    }

    let comment_span = context.span(comment_id);
    let comment_token_type = context
        .first_non_trivia_token_in_span(comment_span)
        .map(|token| token.token.ty);

    matches!(
        comment_token_type,
        Some(TokenType::LineComment | TokenType::DocLineComment | TokenType::DocBlockComment)
    )
}

/// Write one colon-prefixed type annotation with group-aware seam layout.
pub(crate) fn write_colon_prefixed_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [token(":"), space()])?;

    write_expression_with_inline_prefix_annotations(f, expression_id)
}

/// Write one static type argument, preserving type-position seam ownership.
fn write_static_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    match f.context().tree.get(argument_id) {
        Argument::Positional { value, .. } => {
            let argument_span = f.context().span(argument_id);
            let value_span = f.context().span(*value);
            if argument_span.file == value_span.file && argument_span.start < value_span.start {
                let comment_ids = f
                    .context()
                    .comment_nodes_in_range(argument_span.start, value_span.start);
                if !comment_ids.is_empty() {
                    write_leading_raw_type_position_comment_nodes(f, &comment_ids)?;
                }
            }

            write_expression_with_inline_prefix_annotations(f, *value)
        }
        _ => write!(f, [argument_id]),
    }
}

/// Write one rhs expression while preserving inline prefix annotation ownership.
pub(crate) fn write_expression_with_inline_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // type grouping root
    let expression_id = if expression_is_type_position(f.context(), expression_id) {
        let mut current_expression_id = expression_id;

        loop {
            let Some((parent_id, parent_type)) = f.context().parent(current_expression_id) else {
                break;
            };
            if parent_type != NodeType::Expression {
                break;
            }

            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            match f.context().tree.get(parent_expression_id) {
                Expression::Parenthesized { expression }
                    if *expression == current_expression_id =>
                {
                    current_expression_id = parent_expression_id;
                }
                Expression::Binary {
                    operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                    left,
                    right,
                } if expression_has_type_grouping_semantics(f.context(), parent_expression_id)
                    && (*left == current_expression_id || *right == current_expression_id) =>
                {
                    current_expression_id = parent_expression_id;
                }
                _ => break,
            }
        }

        current_expression_id
    } else {
        expression_id
    };
    let leading_raw_comment_ids =
        if expression_has_type_grouping_semantics(f.context(), expression_id) {
            Vec::new()
        } else {
            leading_raw_type_position_comment_nodes(f.context(), expression_id)
        };

    // leading separator
    let leading_separator_token_type =
        if expression_has_type_grouping_semantics(f.context(), expression_id)
            && !matches!(
                f.context().tree.get(expression_id),
                Expression::Binary { .. } | Expression::ReferenceOf { .. }
            )
        {
            let span = f.context().span(expression_id);
            f.context()
                .first_non_trivia_token_in_span(span)
                .and_then(|first_token| match first_token.token.ty {
                    TokenType::ElementwiseOr | TokenType::ElementwiseAnd => {
                        Some(first_token.token.ty)
                    }
                    _ => None,
                })
        } else {
            None
        };

    // prefix annotations
    let mut prefix_annotation_ids = Vec::new();
    let mut saw_inline_non_slash_prefix_annotation = false;
    let mut saw_disqualifying_prefix_annotation = false;

    for annotation_id in f.context().annotation_ids(expression_id).iter().copied() {
        let annotation = f.context().annotation(annotation_id);
        if !matches!(
            annotation.position(),
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            continue;
        }

        prefix_annotation_ids.push(annotation_id);

        if f.context().annotation_starts_on_own_line(annotation_id) {
            saw_disqualifying_prefix_annotation = true;
            continue;
        }

        if matches!(
            annotation,
            Annotation::Doc { node, .. }
                if f.context().tree.get::<Doc>(node).style == DocStyle::Slash
        ) {
            saw_disqualifying_prefix_annotation = true;
            continue;
        }

        saw_inline_non_slash_prefix_annotation = true;
    }

    let has_inline_non_slash_prefix_annotations =
        saw_inline_non_slash_prefix_annotation && !saw_disqualifying_prefix_annotation;
    let expression_is_type_union_root =
        binary_like_is_type_union(f.context(), expression_id, BinaryOperator::ElementwiseOr);
    let suppress_leading_raw_comment_owner = !expression_is_type_union_root
        && !leading_raw_comment_ids.is_empty()
        && leading_raw_comment_ids
            .iter()
            .copied()
            .all(|comment_id| !type_position_comment_requires_break_after(f.context(), comment_id));

    let write_expression_body = |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
        let should_skip_generic_prefix_owner =
            expression_is_type_union_root && leading_separator_token_type.is_none();

        // leading separator
        if let Some(leading_separator_token_type) = leading_separator_token_type {
            // inline prefix comments stay with the separator
            if has_inline_non_slash_prefix_annotations {
                write_annotation_prefix_sequence(f, &prefix_annotation_ids)?;
                write!(f, [space()])?;
            } else if !prefix_annotation_ids.is_empty() {
                write!(
                    f,
                    [crate::format::annotation::prefix_annotations(
                        f.context(),
                        expression_id
                    )]
                )?;

                let last_prefix_annotation_id = *prefix_annotation_ids
                    .last()
                    .expect("non-empty prefix annotation list");
                if f.context()
                    .annotation_next_token_is_on_same_line(last_prefix_annotation_id)
                {
                    write!(f, [space()])?;
                }
            }

            match leading_separator_token_type {
                TokenType::ElementwiseOr => write!(f, [token("|"), space()])?,
                TokenType::ElementwiseAnd => write!(f, [token("&")])?,
                _ => unreachable!("unexpected leading separator token"),
            }

            if suppress_leading_raw_comment_owner {
                f.context()
                    .push_owned_comment_nodes(&leading_raw_comment_ids);
            }

            let result = write_expression_without_prefix_annotations(f, expression_id);

            if suppress_leading_raw_comment_owner {
                f.context()
                    .pop_owned_comment_nodes(leading_raw_comment_ids.len());
            }

            result?;

            return Ok(());
        }

        // default type-position owner
        if !has_inline_non_slash_prefix_annotations {
            if suppress_leading_raw_comment_owner {
                f.context()
                    .push_owned_comment_nodes(&leading_raw_comment_ids);
            }

            let result = if should_skip_generic_prefix_owner {
                write_expression_without_prefix_annotations(f, expression_id)
            } else {
                write!(f, [expression_id])
            };

            if suppress_leading_raw_comment_owner {
                f.context()
                    .pop_owned_comment_nodes(leading_raw_comment_ids.len());
            }

            result?;
            return Ok(());
        }

        // inline prefix annotations
        write_annotation_prefix_sequence(f, &prefix_annotation_ids)?;
        write!(f, [space()])?;
        if suppress_leading_raw_comment_owner {
            f.context()
                .push_owned_comment_nodes(&leading_raw_comment_ids);
        }

        let result = write_expression_without_prefix_annotations(f, expression_id);

        if suppress_leading_raw_comment_owner {
            f.context()
                .pop_owned_comment_nodes(leading_raw_comment_ids.len());
        }

        result
    };

    // inline union with inline prefix annotations
    if !suppress_leading_raw_comment_owner
        && has_inline_non_slash_prefix_annotations
        && leading_separator_token_type.is_none()
    {
        let union_root_id = transparent_type_binary_root_expression(
            f.context(),
            expression_id,
            BinaryOperator::ElementwiseOr,
        );

        if binary_like_is_type_union(f.context(), union_root_id, BinaryOperator::ElementwiseOr) {
            let operands = flatten_binary_like_operands(
                f.context(),
                union_root_id,
                BinaryOperator::ElementwiseOr,
            );
            let write_inline_union = format_with(|f| {
                write_annotation_prefix_sequence(f, &prefix_annotation_ids)?;
                write!(f, [space()])?;
                format_inline_type_union_layout(f, &operands)
            });
            let write_body = format_with(write_expression_body);

            write!(
                f,
                [destack_fir::best_fitting![write_inline_union, write_body]
                    .with_mode(destack_fir::format::BestFittingMode::AllLines)]
            )?;
            return Ok(());
        }
    }

    // inline raw type-position comments
    if suppress_leading_raw_comment_owner {
        let union_root_id = transparent_type_binary_root_expression(
            f.context(),
            expression_id,
            BinaryOperator::ElementwiseOr,
        );

        if prefix_annotation_ids.is_empty()
            && leading_separator_token_type.is_none()
            && binary_like_is_type_union(f.context(), union_root_id, BinaryOperator::ElementwiseOr)
        {
            let operands = flatten_binary_like_operands(
                f.context(),
                union_root_id,
                BinaryOperator::ElementwiseOr,
            );
            let write_inline_union = format_with(|f| {
                write_leading_raw_type_position_comment_nodes(f, &leading_raw_comment_ids)?;
                f.context()
                    .push_owned_comment_nodes(&leading_raw_comment_ids);
                let result = format_inline_type_union_layout(f, &operands);
                f.context()
                    .pop_owned_comment_nodes(leading_raw_comment_ids.len());
                result
            });
            let write_body = format_with(|f| {
                write_leading_raw_type_position_comment_nodes(f, &leading_raw_comment_ids)?;
                f.context()
                    .push_owned_comment_nodes(&leading_raw_comment_ids);
                let result = write_expression_body(f);
                f.context()
                    .pop_owned_comment_nodes(leading_raw_comment_ids.len());
                result
            });

            write!(
                f,
                [destack_fir::best_fitting![write_inline_union, write_body]
                    .with_mode(destack_fir::format::BestFittingMode::AllLines)]
            )?;
            return Ok(());
        }

        let write_body = format_with(|f| {
            write_leading_raw_type_position_comment_nodes(f, &leading_raw_comment_ids)?;
            f.context()
                .push_owned_comment_nodes(&leading_raw_comment_ids);
            let result = write_expression_body(f);
            f.context()
                .pop_owned_comment_nodes(leading_raw_comment_ids.len());
            result
        });
        let interned_body = f.intern(&write_body)?;
        let write_flat_body = format_with(move |f| {
            if let Some(interned_body) = &interned_body {
                let mut buffer = RemoveSoftLinesBuffer::new(f);
                buffer.write_node(interned_body.clone());
            }

            Ok(())
        });

        write!(
            f,
            [destack_fir::best_fitting![write_flat_body, write_body]
                .with_mode(destack_fir::format::BestFittingMode::AllLines)]
        )?;
        return Ok(());
    }

    // breaking raw type-position comments
    if !leading_raw_comment_ids.is_empty() {
        write_leading_raw_type_position_comment_nodes(f, &leading_raw_comment_ids)?;
        f.context()
            .push_owned_comment_nodes(&leading_raw_comment_ids);
        let result = write_expression_body(f);
        f.context()
            .pop_owned_comment_nodes(leading_raw_comment_ids.len());
        return result;
    }

    write_expression_body(f)
}

/// Format static type arguments without multiline trailing commas.
pub(crate) fn format_static_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if static_argument_list_is_hug_safe(f.context(), static_arguments) {
        write!(f, [token("<")])?;
        for (index, argument_id) in static_arguments.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            write_static_argument(f, *argument_id)?;
        }
        write!(f, [token(">")])?;
        return Ok(());
    }

    let should_expand = should_expand_static_argument_list(f.context(), static_arguments);
    let has_line_postfix_boundary = static_arguments.iter().copied().any(|argument_id| {
        f.context()
            .annotation_ids(argument_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                matches!(
                    f.context().annotation(annotation_id),
                    Annotation::Doc {
                        position: AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
    });
    let format_arguments = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (index, argument_id) in static_arguments.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(","), soft_line_break_or_space()])?;
            }

            write_static_argument(f, argument_id)?;
        }

        Ok(())
    });
    write!(
        f,
        [group(&format_args![
            token("<"),
            soft_block_indent(&format_arguments),
            token(">")
        ])
        .should_expand(should_expand || has_line_postfix_boundary)]
    )
}

/// Return whether one preserved parenthesized type expression prefers soft-block layout.
pub(crate) fn parenthesized_type_expression_prefers_soft_block_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
    has_parenthesized_leading_inner_comments: bool,
    inner_has_effective_prefix_annotation: bool,
) -> bool {
    let inner_expression = context.tree.get(expression_id);
    let inner_expression_has_comments = {
        let expression_span = context.span(expression_id);
        !context
            .comments_in_range(expression_span.start, expression_span.end)
            .is_empty()
    };
    let parenthesized_starts_with_type_operator = {
        if !parenthesized_has_explicit_delimiters(context, node_id, expression_id) {
            false
        } else {
            let parenthesized_span = context.span(node_id);
            let inner_span = context.span(expression_id);
            let mut search_start = parenthesized_span.start.saturating_add(1);

            if search_start >= inner_span.end || parenthesized_span.file != inner_span.file {
                false
            } else {
                loop {
                    let Some(token) =
                        context.first_non_trivia_token_between(search_start, inner_span.end)
                    else {
                        break false;
                    };

                    if token.token.ty == TokenType::OpenParenthesis {
                        if token.span.end <= search_start {
                            break false;
                        }

                        search_start = token.span.end;
                        continue;
                    }

                    break matches!(
                        token.token.ty,
                        TokenType::ElementwiseOr | TokenType::ElementwiseAnd
                    );
                }
            }
        }
    };
    let union_or_intersection_member_count = {
        let mut count = 0usize;
        let mut stack = vec![expression_id];

        while let Some(current_id) = stack.pop() {
            match context.tree.get(current_id) {
                Expression::Binary {
                    operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                    left,
                    right,
                } => {
                    stack.push(*right);
                    stack.push(*left);
                }
                _ => count += 1,
            }
        }

        count
    };

    (parenthesized_starts_with_type_operator && inner_expression_has_comments)
        || (matches!(
            inner_expression,
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                ..
            }
        ) && (inner_has_effective_prefix_annotation
            || has_parenthesized_leading_inner_comments
            || inner_expression_has_comments))
        || union_or_intersection_member_count > 2
}

/// Decide whether a parenthesized type expression can drop wrappers.
pub(crate) fn should_drop_parenthesized_type_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let has_non_grouping_wrapper_annotation = context.has_annotation(node_id)
        && !expression_has_only_type_grouping_prefix_annotations(context, node_id);
    if has_non_grouping_wrapper_annotation {
        return false;
    }

    let has_non_grouping_inner_annotation = context.has_annotation(inner_id)
        && !expression_has_only_type_grouping_prefix_annotations(context, inner_id);

    if parenthesized_wraps_decorated_class_extends_head(context, node_id, inner_id) {
        return false;
    }

    if parenthesized_wraps_prefix_annotated_class_extends_head(context, node_id, inner_id) {
        return false;
    }

    let is_parenthesized_type_position = expression_is_type_position(context, node_id)
        || expression_is_type_position(context, inner_id);
    if !is_parenthesized_type_position {
        return false;
    }

    let normalized_inner_id = transparent_inner_expression(
        context,
        normalize_parenthesized_type_grouping_inner_expression(context, inner_id),
    );
    let inner_is_single_operand_type_binary = matches!(
        context.tree.get(normalized_inner_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
            ..
        }
    ) && flatten_type_binary_expression(
        context,
        normalized_inner_id,
        match context.tree.get(normalized_inner_id) {
            Expression::Binary { operator, .. } => *operator,
            _ => unreachable!("checked above"),
        },
    )
    .len()
        == 1;

    let can_drop_array_element_wrapper =
        context
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }

                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                if let Expression::Index { left, index, .. } =
                    context.tree.get(parent_expression_id)
                {
                    *left == node_id
                        && index.is_none()
                        && !context.has_annotation(normalized_inner_id)
                        && (is_simple_type_binary_left_expression(
                            context.tree,
                            normalized_inner_id,
                        ) || inner_is_single_operand_type_binary)
                } else {
                    false
                }
            });
    let can_drop_associative_type_binary_wrapper = {
        match context.tree.get(inner_id) {
            Expression::Binary {
                operator: inner_operator,
                ..
            } if is_associative_type_binary_operator(*inner_operator) => context
                .parent(node_id)
                .is_some_and(|(parent_id, parent_type)| {
                    if parent_type != NodeType::Expression {
                        return false;
                    }

                    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                    match context.tree.get(parent_expression_id) {
                        Expression::Binary {
                            left,
                            operator,
                            right,
                        } => {
                            (*left == node_id || *right == node_id)
                                && *operator == *inner_operator
                                && expression_has_type_grouping_semantics(
                                    context,
                                    parent_expression_id,
                                )
                        }
                        _ => false,
                    }
                }),
            _ => false,
        }
    };
    let can_drop_conditional_type_grouping = matches!(
        context.tree.get(normalized_inner_id),
        Expression::TypeConditional { .. }
    ) && context.parent(node_id).is_some_and(
        |(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            match context.tree.get(parent_expression_id) {
                Expression::Binary {
                    left,
                    operator,
                    right,
                } => {
                    (*left == node_id || *right == node_id)
                        && matches!(
                            operator,
                            BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                        )
                        && expression_has_type_grouping_semantics(context, parent_expression_id)
                }
                _ => false,
            }
        },
    );
    let can_drop_single_operand_grouping =
        context
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }

                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                let Expression::Binary { operator, .. } = context.tree.get(parent_expression_id)
                else {
                    return false;
                };
                if !matches!(
                    operator,
                    BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                ) || !expression_has_type_grouping_semantics(context, parent_expression_id)
                {
                    return false;
                }

                let operands =
                    flatten_type_binary_expression(context, parent_expression_id, *operator);
                operands.len() == 1
            });

    if can_drop_array_element_wrapper
        || can_drop_associative_type_binary_wrapper
        || can_drop_conditional_type_grouping
        || can_drop_single_operand_grouping
        || inner_is_single_operand_type_binary
    {
        return true;
    }

    let parenthesized_type_grouping_drop_is_safe_in_parent =
        context
            .parent(node_id)
            .is_none_or(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return true;
                }

                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                match context.tree.get(parent_expression_id) {
                    Expression::Index { left, .. } | Expression::TypeIndex { left, .. } => {
                        *left != node_id
                    }
                    Expression::Binary {
                        left,
                        operator: parent_operator,
                        right,
                    } if (*left == node_id || *right == node_id)
                        && expression_has_type_grouping_semantics(
                            context,
                            parent_expression_id,
                        ) =>
                    {
                        let Expression::Binary {
                            operator: inner_operator,
                            ..
                        } = context.tree.get(inner_id)
                        else {
                            return true;
                        };

                        !(is_associative_type_binary_operator(*parent_operator)
                            && is_associative_type_binary_operator(*inner_operator)
                            && parent_operator != inner_operator)
                    }
                    _ => true,
                }
            });
    if !parenthesized_type_grouping_drop_is_safe_in_parent {
        return false;
    }

    let parenthesized_root_associative_type_binary_can_drop = {
        matches!(
            context.tree.get(normalized_inner_id),
            Expression::Binary { operator, .. }
                if is_associative_type_binary_operator(*operator)
                    && expression_has_type_grouping_semantics(context, normalized_inner_id)
        )
    };
    if parenthesized_root_associative_type_binary_can_drop {
        return true;
    }

    let parenthesized_type_alias_value_can_drop =
        context
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Declaration {
                    return false;
                }

                let declaration_id = LocalNodeId::<destack_ast::Declaration>::new(parent_id);
                matches!(
                    context.tree.get(declaration_id),
                    destack_ast::Declaration::Type { value, .. } if *value == node_id
                )
            })
            && matches!(
                context.tree.get(normalized_inner_id),
                Expression::Binary { operator, .. }
                    if is_associative_type_binary_operator(*operator)
            );
    if parenthesized_type_alias_value_can_drop {
        return true;
    }

    if has_non_grouping_inner_annotation {
        return false;
    }

    let parent = context.parent(node_id);
    let parent_is_expression =
        parent.is_some_and(|(_, parent_type)| parent_type == NodeType::Expression);
    let parent_is_function_return_type = parent.is_some_and(|(parent_id, parent_type)| {
        if parent_type != NodeType::Declaration {
            return false;
        }

        let declaration_id = LocalNodeId::<destack_ast::Declaration>::new(parent_id);
        matches!(
            context.tree.get(declaration_id),
            destack_ast::Declaration::Function { signature, .. } if signature.return_type == Some(node_id)
        )
    });
    let parent_is_type_conditional_arm = parent.is_some_and(|(parent_id, parent_type)| {
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        matches!(
            context.tree.get(parent_expression_id),
            Expression::TypeConditional { left, right, .. }
                if *left == node_id || *right == node_id
        )
    });

    let inner_is_lambda_declaration = matches!(
        context.tree.get(inner_id),
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                destack_ast::Declaration::Function { signature, .. } if signature.kind == destack_ast::FunctionKind::Lambda
            )
    );
    let inner_is_type_conditional = matches!(
        context.tree.get(normalized_inner_id),
        Expression::TypeConditional { .. }
    );
    let inner_is_simple_type_binary_left =
        is_simple_type_binary_left_expression(context.tree, inner_id);

    if !parent_is_expression && inner_is_type_conditional {
        return true;
    }

    if inner_is_lambda_declaration
        && !parent_is_function_return_type
        && !parent_is_type_conditional_arm
    {
        return true;
    }

    inner_is_simple_type_binary_left
}

/// Format static type arguments with relational spacing for index-following instantiations.
pub(crate) fn format_static_argument_list_with_relational_spacing<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [space(), token("<"), space()])?;
    for (index, argument_id) in static_arguments.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [*argument_id])?;
    }
    write!(f, [space(), token(">"), space()])
}

/// Return static argument slots for expression variants that support type arguments.
pub(crate) fn expression_static_arguments(
    expression: &Expression,
) -> Option<&[LocalNodeId<Argument>]> {
    match expression {
        Expression::QualifiedReference {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        }
        | Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        }
        | Expression::TypeImport {
            static_arguments, ..
        } => static_arguments.as_deref(),
        Expression::Instantiation {
            static_arguments, ..
        } => Some(static_arguments.as_slice()),
        _ => None,
    }
}

/// Return whether an expression tree contains static type arguments.
pub(crate) fn expression_has_static_type_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::QualifiedReference {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || expression_has_static_type_arguments(context, *left)
        }
        Expression::Call {
            left,
            static_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            ..
        } => {
            static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || expression_has_static_type_arguments(context, *left)
        }
        Expression::Instantiation {
            left,
            static_arguments,
        } => !static_arguments.is_empty() || expression_has_static_type_arguments(context, *left),
        Expression::Parenthesized { expression } => {
            expression_has_static_type_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether one expression sits in one explicit or nested type position.
pub(crate) fn expression_is_type_position(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if context.expression_is_under_forced_type_position_root(node_id) {
        return true;
    }

    let mut current_id = node_id.id;

    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        let current_expression_id = LocalNodeId::<Expression>::new(current_id);
        let is_explicit_type_root = match parent_type {
            NodeType::Argument => {
                let argument_id = LocalNodeId::<Argument>::new(parent_id);
                let Some((expression_id, expression_type)) = context.parent(argument_id) else {
                    return false;
                };
                if expression_type != NodeType::Expression {
                    return false;
                }

                let parent_expression = context
                    .tree
                    .get(LocalNodeId::<Expression>::new(expression_id));
                expression_static_arguments(parent_expression)
                    .is_some_and(|arguments| arguments.contains(&argument_id))
            }
            NodeType::Declarator => {
                let declarator = context
                    .tree
                    .get(LocalNodeId::<destack_ast::Declarator>::new(parent_id));
                declarator.ty.is_some_and(|ty| ty == current_expression_id)
            }
            NodeType::Parameter => {
                let parameter = context
                    .tree
                    .get(LocalNodeId::<destack_ast::Parameter>::new(parent_id));
                let parameter_ty = match parameter {
                    destack_ast::Parameter::Named { ty, .. }
                    | destack_ast::Parameter::Pattern { ty, .. }
                    | destack_ast::Parameter::VariadicNamed { ty, .. }
                    | destack_ast::Parameter::VariadicPattern { ty, .. } => *ty,
                    destack_ast::Parameter::Error => None,
                };

                parameter_ty.is_some_and(|ty| ty == current_expression_id)
            }
            NodeType::WhereClause => {
                let where_clause = context
                    .tree
                    .get(LocalNodeId::<destack_ast::WhereClause>::new(parent_id));
                where_clause.right == current_expression_id
            }
            NodeType::Declaration => {
                let declaration = context
                    .tree
                    .get(LocalNodeId::<destack_ast::Declaration>::new(parent_id));
                match declaration {
                    destack_ast::Declaration::Type { value, .. } => *value == current_expression_id,
                    destack_ast::Declaration::Struct { heritage, .. }
                    | destack_ast::Declaration::Interface { heritage, .. }
                    | destack_ast::Declaration::Enum { heritage, .. } => {
                        heritage
                            .extends_types
                            .as_ref()
                            .is_some_and(|types| types.contains(&current_expression_id))
                            || heritage
                                .implements_types
                                .as_ref()
                                .is_some_and(|types| types.contains(&current_expression_id))
                    }
                    destack_ast::Declaration::Class { heritage, .. } => heritage
                        .implements_types
                        .as_ref()
                        .is_some_and(|types| types.contains(&current_expression_id)),
                    destack_ast::Declaration::Extension {
                        target_type,
                        heritage,
                        ..
                    } => {
                        *target_type == current_expression_id
                            || heritage
                                .extends_types
                                .as_ref()
                                .is_some_and(|types| types.contains(&current_expression_id))
                            || heritage
                                .implements_types
                                .as_ref()
                                .is_some_and(|types| types.contains(&current_expression_id))
                    }
                    destack_ast::Declaration::Function { signature, .. } => signature
                        .return_type
                        .is_some_and(|return_type| return_type == current_expression_id),
                    destack_ast::Declaration::ImportAlias { kind, target, .. } => {
                        matches!(
                            (kind, target),
                            (
                                destack_ast::DependencyKind::Type,
                                destack_ast::ImportAliasTarget::Path { value }
                            ) if *value == current_expression_id
                        )
                    }
                    destack_ast::Declaration::Global { .. }
                    | destack_ast::Declaration::Namespace { .. } => false,
                }
            }
            _ => false,
        };
        if is_explicit_type_root {
            return true;
        }

        match parent_type {
            NodeType::Expression => {
                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                let parent_expression = context.tree.get(parent_expression_id);

                if let Expression::TypeUnary { operator, right } = parent_expression
                    && *operator == TypeUnaryOperator::AsConst
                    && right.id == current_id
                {
                    current_id = parent_id;
                    continue;
                }

                if let Expression::Index { left, index, .. } = parent_expression
                    && *left == current_expression_id
                    && index.is_none()
                {
                    current_id = parent_id;
                    continue;
                }

                if let Expression::TypeBinary { left, operator, .. } = parent_expression
                    && left.id == current_id
                    && matches!(
                        operator,
                        TypeBinaryOperator::Cast
                            | TypeBinaryOperator::Satisfies
                            | TypeBinaryOperator::Is
                            | TypeBinaryOperator::InstanceOf
                            | TypeBinaryOperator::In
                    )
                {
                    current_id = parent_id;
                    continue;
                }

                if let Expression::Binary { left, right, .. } = parent_expression
                    && (left.id == current_id || right.id == current_id)
                    && expression_has_type_grouping_semantics(context, parent_expression_id)
                {
                    current_id = parent_id;
                    continue;
                }

                if matches!(
                    parent_expression,
                    Expression::TypeUnary { .. }
                        | Expression::TypeBinary { .. }
                        | Expression::TypeConditional { .. }
                        | Expression::TypeMapped { .. }
                        | Expression::TypeIndex { .. }
                        | Expression::TypeTemplateLiteral { .. }
                        | Expression::TypeImport { .. }
                        | Expression::TypeInfer { .. }
                        | Expression::TypePredicate { .. }
                ) {
                    return true;
                }

                return false;
            }
            NodeType::Property => {
                let property = context.tree.get(LocalNodeId::<Property>::new(parent_id));
                if let Property::Field { value, .. } = property
                    && value.is_some_and(|value| value.id == current_id)
                    && let Some((expression_id, expression_type)) = context.parent_by_id(parent_id)
                    && expression_type == NodeType::Expression
                {
                    return expression_is_type_position(
                        context,
                        LocalNodeId::<Expression>::new(expression_id),
                    );
                }

                return false;
            }
            NodeType::Member => {
                let member = context.tree.get(LocalNodeId::<Member>::new(parent_id));
                return match member {
                    Member::Type { ty, value, .. } => {
                        ty.is_some_and(|ty| ty.id == current_id)
                            || value.is_some_and(|value| value.id == current_id)
                    }
                    Member::Field { value, .. } => {
                        value.is_some_and(|value| value.id == current_id)
                    }
                    Member::ComptimeConst { ty, .. } => ty.is_some_and(|ty| ty.id == current_id),
                    Member::Embed { value, .. } => value.id == current_id,
                    Member::Method { .. }
                    | Member::StaticBlock { .. }
                    | Member::ComptimeBlock { .. }
                    | Member::Error => false,
                };
            }
            _ => return false,
        }
    }

    false
}

/// Return whether one expression participates in `|` or `&` type grouping semantics.
pub(crate) fn expression_has_type_grouping_semantics(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
            ..
        }
    ) && expression_is_type_position(context, expression_id)
}
