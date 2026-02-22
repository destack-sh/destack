use crate::format::chain::{
    Annotation, AnnotationPosition, ChainExpression, DestackFormatContext, Expression, IfKind,
    LocalNodeId, NodeTree, NodeType, PostfixPosition, Span, TokenType, argument_forces_multiline,
    argument_is_function_expression, argument_is_inline_closure_cast_object,
    assignment_like_parent, chain_call_can_expand_in_head,
    chain_has_parent_intervening_break_or_comment, chain_head_id, expression_trivia_anchor_end,
    has_comment_between_expressions, is_call_like_argument, is_chain_expression,
    is_nested_lambda_expression, is_numeric_index, is_simple_chain_argument,
    is_simple_chain_operation, is_simple_chain_static_arguments,
    member_has_intervening_break_or_comment, member_has_intervening_comment, span_has_comment,
};
use crate::format::expression::TypeBinaryOperator;
use destack_ast::{Comment, CommentStyle, Doc, DocStyle};

/// Summarize the complexity of a call within a chain.
pub(crate) struct ChainCallSummary {
    pub(crate) has_multiline_argument: bool,
    pub(crate) has_non_simple_argument: bool,
}

/// Build call summaries for a chain in source order.
pub(crate) fn summarize_chain_calls(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> Vec<ChainCallSummary> {
    let mut summaries = Vec::new();

    for expression_id in chain {
        let Expression::Call {
            static_arguments,
            dynamic_arguments,
            ..
        } = context.tree.get(*expression_id)
        else {
            continue;
        };

        // collect per-call signals
        let has_multiline_argument = dynamic_arguments
            .iter()
            .copied()
            .any(|argument_id| argument_forces_multiline(context, argument_id))
            || static_arguments.as_ref().is_some_and(|arguments| {
                arguments
                    .iter()
                    .copied()
                    .any(|argument_id| argument_forces_multiline(context, argument_id))
            });
        let has_non_simple_argument = static_arguments.as_ref().is_some_and(|arguments| {
            arguments
                .iter()
                .copied()
                .any(|argument_id| !is_simple_chain_argument(context, argument_id))
        });
        summaries.push(ChainCallSummary {
            has_multiline_argument,
            has_non_simple_argument,
        });
    }

    summaries
}

/// Return whether the chain root is a path with multiple tail segments.
pub(crate) fn chain_root_has_path_tail_segments(
    context: &DestackFormatContext<'_>,
    chain_root: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(chain_root),
        Expression::Path { path, .. } if path.segments.len() > 1
    )
}

/// Return whether any chain link carries an optional-style tail.
pub(crate) fn chain_has_optional_tail(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Maybe { .. }
                | Expression::Call {
                    position: PostfixPosition::Indirect,
                    ..
                }
        )
    })
}

/// Return whether any chain link is a member-style access.
pub(crate) fn chain_has_member_access(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
    })
}

/// Return whether adjacent chain links form direct curried calls.
pub(crate) fn chain_has_direct_curried_call_pair(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.windows(2).any(|pair| {
        pair.iter().all(|expression_id| {
            matches!(
                context.tree.get(*expression_id),
                Expression::Call {
                    position: PostfixPosition::Direct,
                    ..
                }
            )
        })
    })
}

/// Return whether one direct call in the chain has an inline closure cast object argument.
pub(crate) fn chain_has_direct_call_with_inline_closure_cast_object_argument(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.iter().copied().any(|expression_id| {
        let Expression::Call {
            position: PostfixPosition::Direct,
            dynamic_arguments,
            ..
        } = context.tree.get(expression_id)
        else {
            return false;
        };

        dynamic_arguments
            .iter()
            .copied()
            .any(|argument_id| argument_is_inline_closure_cast_object(context, argument_id))
    })
}

/// Return whether a non-head call in a chain takes a non-lambda function argument.
pub(crate) fn chain_has_nonhead_nonlambda_function_call_argument(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    let mut call_index = 0usize;

    for expression_id in chain {
        let Expression::Call {
            dynamic_arguments, ..
        } = context.tree.get(*expression_id)
        else {
            continue;
        };

        if call_index > 0
            && dynamic_arguments
                .iter()
                .copied()
                .any(|argument_id| argument_is_function_expression(context, argument_id))
        {
            return true;
        }

        call_index += 1;
    }

    false
}

/// Return whether a parent call on the chain tail should force dot-level breaking.
pub(crate) fn chain_tail_parent_call_requires_chain_break(
    context: &DestackFormatContext<'_>,
    chain_tail: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(chain_tail) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Call {
        left,
        static_arguments,
        dynamic_arguments,
        ..
    } = context.tree.get(parent_expression_id)
    else {
        return false;
    };
    if *left != chain_tail {
        return false;
    }

    !is_simple_chain_static_arguments(context, static_arguments) && !dynamic_arguments.is_empty()
}

/// Return whether chain annotations should force breaking.
pub(crate) fn chain_has_breaking_annotations(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
    chain_head: LocalNodeId<Expression>,
) -> bool {
    // root prefix annotations are statement-level concerns:
    // root non-prefix annotation handling stays in `chain_head_has_non_prefix_breaking_annotation`
    let has_chain_node_annotations = chain
        .iter()
        .copied()
        .skip(1)
        .any(|expression_id| chain_node_has_breaking_annotation(context, expression_id));
    let chain_head_has_non_prefix_breaking_annotation =
        chain_head_has_non_prefix_breaking_annotation(context, chain_head);

    has_chain_node_annotations || chain_head_has_non_prefix_breaking_annotation
}

/// Return whether a chain head has non-prefix breaking annotation positions.
fn chain_head_has_non_prefix_breaking_annotation(
    context: &DestackFormatContext<'_>,
    chain_head: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(chain_head, |annotations| {
            annotations.iter().any(|annotation_id| {
                if chain_annotation_is_inline_non_breaking(context, *annotation_id)
                    || chain_annotation_is_internal_call_argument_infix(
                        context,
                        chain_head,
                        *annotation_id,
                    )
                {
                    return false;
                }

                let position = context.annotation(*annotation_id).position();
                if matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return false;
                }

                matches!(
                    position,
                    AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether annotations or intervening trivia should force a break.
pub(crate) fn should_break_for_annotation_or_trivia(
    has_chain_annotations: bool,
    has_chain_intervening_comment: bool,
    has_member_access: bool,
) -> bool {
    has_chain_annotations || (has_chain_intervening_comment && has_member_access)
}

/// Return whether path roots with optional tails and trivia should force a break.
pub(crate) fn should_break_for_path_optional_tail(
    has_chain_intervening_trivia: bool,
    root_has_path_tail_segments: bool,
    has_optional_tail: bool,
) -> bool {
    has_chain_intervening_trivia && root_has_path_tail_segments && has_optional_tail
}

/// Return whether curried direct calls with intervening trivia should force a break.
pub(crate) fn should_break_for_curried_call_intervening_trivia(
    has_chain_intervening_trivia: bool,
    has_direct_curried_call_pair: bool,
) -> bool {
    has_chain_intervening_trivia && has_direct_curried_call_pair
}

/// Return whether non-head calls have multiline or non-simple arguments.
pub(crate) fn should_break_for_nonhead_call_complexity(
    call_summaries: &[ChainCallSummary],
) -> bool {
    let has_nonhead_multiline_call_argument = call_summaries
        .iter()
        .skip(1)
        .any(|summary| summary.has_multiline_argument);
    if has_nonhead_multiline_call_argument {
        return true;
    }

    call_summaries
        .iter()
        .skip(1)
        .any(|summary| summary.has_non_simple_argument)
}

/// Collect break-relevant signals for one chain.
pub(crate) struct ChainBreakAnalysis {
    pub(crate) should_break: bool,
    pub(crate) call_summaries: Vec<ChainCallSummary>,
    pub(crate) has_chain_intervening_trivia: bool,
}

/// Build chain break signals once so callers can reuse them.
pub(crate) fn analyze_chain_break(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> ChainBreakAnalysis {
    if chain.is_empty() {
        return ChainBreakAnalysis {
            should_break: false,
            call_summaries: Vec::new(),
            has_chain_intervening_trivia: false,
        };
    }

    let chain_root = chain[0];
    let chain_tail = chain[chain.len() - 1];
    let chain_head = chain_head_id(context.tree, chain_root);
    let call_summaries = summarize_chain_calls(context, chain);
    let has_chain_intervening_trivia = chain_has_intervening_break_or_comment(context, chain);
    let has_chain_intervening_comment = chain_has_intervening_comment(context, chain);
    let root_has_path_tail_segments = chain_root_has_path_tail_segments(context, chain_root);
    let has_optional_tail = chain_has_optional_tail(context, chain);
    let has_member_access = chain_has_member_access(context, chain);
    let has_chain_annotations = chain_has_breaking_annotations(context, chain, chain_head);
    let has_direct_curried_call_pair = chain_has_direct_curried_call_pair(context, chain);
    let has_direct_call_with_inline_closure_cast_object_argument =
        chain_has_direct_call_with_inline_closure_cast_object_argument(context, chain);

    let should_break = if should_break_for_annotation_or_trivia(
        has_chain_annotations,
        has_chain_intervening_comment,
        has_member_access,
    ) {
        true
    } else if should_break_for_path_optional_tail(
        has_chain_intervening_trivia,
        root_has_path_tail_segments,
        has_optional_tail,
    ) {
        true
    } else if should_break_for_curried_call_intervening_trivia(
        has_chain_intervening_trivia,
        has_direct_curried_call_pair,
    ) {
        true
    } else if has_direct_call_with_inline_closure_cast_object_argument {
        true
    } else if call_summaries.is_empty() {
        false
    } else if should_break_for_nonhead_call_complexity(&call_summaries) {
        true
    } else if chain_tail_parent_call_requires_chain_break(context, chain_tail) {
        true
    } else if call_summaries.len() == 1 {
        false
    } else {
        analyze_chain_parent_facts(context, chain_tail).chain_overflows_in_type_binary_left()
    };

    ChainBreakAnalysis {
        should_break,
        call_summaries,
        has_chain_intervening_trivia,
    }
}

/// Decide whether a chain should proactively break across lines.
pub(crate) fn should_break_chain(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    analyze_chain_break(context, chain).should_break
}

/// Return whether the next non-whitespace token after one annotation starts on the same line.
fn annotation_next_token_is_on_same_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = context.annotation_span(annotation_id);
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.start < span.end);

    while let Some(token) = tokens.get(index).copied() {
        match token.token.ty {
            TokenType::Whitespace => {
                index += 1;
                continue;
            }
            TokenType::Newline => return false,
            _ => return true,
        }
    }

    false
}

/// Return the first non-whitespace token kind after one annotation.
fn annotation_next_non_whitespace_token_type(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<TokenType> {
    let span = context.annotation_span(annotation_id);
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.start < span.end);

    while let Some(token) = tokens.get(index).copied() {
        match token.token.ty {
            TokenType::Whitespace | TokenType::Newline => {
                index += 1;
                continue;
            }
            token_type => return Some(token_type),
        }
    }

    None
}

/// Return whether one annotation should not force multiline chain breaking.
pub(crate) fn chain_annotation_is_inline_non_breaking(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = context.annotation(annotation_id);
    let position = annotation.position();
    if !matches!(
        position,
        AnnotationPosition::BlockPrefix
            | AnnotationPosition::BlockPostfix
            | AnnotationPosition::LinePrefix
            | AnnotationPosition::LinePostfix
            | AnnotationPosition::LinePostfixBoundary
    ) {
        return false;
    }

    let is_star_style = match annotation {
        Annotation::Comment { node, .. } => {
            let comment = context.tree.get::<Comment>(node);
            comment.style == CommentStyle::Star
        }
        Annotation::Doc { node, .. } => {
            let doc = context.tree.get::<Doc>(node);
            doc.style == DocStyle::Star
        }
        Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
    };
    if !is_star_style {
        return false;
    }

    let annotation_span = context.annotation_span(annotation_id);
    if context.has_newline(annotation_span) {
        return false;
    }

    if position == AnnotationPosition::LinePostfixBoundary {
        return true;
    }

    if position == AnnotationPosition::LinePostfix
        && annotation_next_non_whitespace_token_type(context, annotation_id)
            == Some(TokenType::Maybe)
    {
        return true;
    }

    annotation_next_token_is_on_same_line(context, annotation_id)
}

/// Return whether one annotation is an internal call argument infix marker.
pub(crate) fn chain_annotation_is_internal_call_argument_infix(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if context.annotation(annotation_id).position() != AnnotationPosition::BlockInfix {
        return false;
    }

    matches!(
        context.tree.get(node_id),
        Expression::Call { .. } | Expression::Instantiation { .. } | Expression::New { .. }
    )
}

/// Return whether one expression is wrapped by a statement expression parent.
fn chain_node_is_statement_wrapped(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::Statement(inner_id) if inner_id.id == node_id.id
            )
        })
}

/// Check whether a chain node has an annotation that should force breaking.
pub(crate) fn chain_node_has_breaking_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let is_chain_link = is_chain_expression(context.tree.get(node_id));
    let is_statement_wrapped_chain_link =
        is_chain_link && chain_node_is_statement_wrapped(context, node_id);
    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                if chain_annotation_is_inline_non_breaking(context, *annotation_id)
                    || chain_annotation_is_internal_call_argument_infix(
                        context,
                        node_id,
                        *annotation_id,
                    )
                {
                    return false;
                }

                let annotation = context.annotation(*annotation_id);
                let position = annotation.position();
                if !is_chain_link
                    && matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    )
                {
                    return false;
                }
                if is_statement_wrapped_chain_link
                    && matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    )
                {
                    return false;
                }

                matches!(
                    position,
                    AnnotationPosition::LinePrefix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockPrefix
                        | AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                )
            })
        })
        .unwrap_or(false)
}

/// Check whether a chain node has annotations that prevent head grouping.
pub(crate) fn chain_node_has_non_inline_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let is_chain_link = is_chain_expression(context.tree.get(node_id));
    let is_statement_wrapped_chain_link =
        is_chain_link && chain_node_is_statement_wrapped(context, node_id);
    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                if chain_annotation_is_inline_non_breaking(context, *annotation_id)
                    || chain_annotation_is_internal_call_argument_infix(
                        context,
                        node_id,
                        *annotation_id,
                    )
                {
                    return false;
                }

                let annotation = context.annotation(*annotation_id);
                let position = annotation.position();
                if !is_chain_link
                    && matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    )
                {
                    return false;
                }
                if is_statement_wrapped_chain_link
                    && matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    )
                {
                    return false;
                }

                match annotation {
                    Annotation::Blank { .. } => true,
                    Annotation::Doc { .. }
                    | Annotation::Comment { .. }
                    | Annotation::Decorator { .. } => matches!(
                        position,
                        AnnotationPosition::LinePrefix
                            | AnnotationPosition::LinePostfixBoundary
                            | AnnotationPosition::BlockPrefix
                            | AnnotationPosition::BlockInfix
                            | AnnotationPosition::BlockPostfix
                    ),
                }
            })
        })
        .unwrap_or(false)
}

/// Check whether a chain line starts with block prefix annotations.
pub(crate) fn chain_line_starts_with_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    line: &[ChainExpression],
) -> bool {
    let Some(first_op) = line.first() else {
        return false;
    };
    let node_id = match first_op {
        ChainExpression::Member { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Instantiation { node_id, .. }
        | ChainExpression::Maybe { node_id, .. }
        | ChainExpression::Must { node_id, .. } => *node_id,
    };

    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id).position(),
                    AnnotationPosition::BlockPrefix
                )
            })
        })
        .unwrap_or(false)
}

/// Record static signals for head split decisions.
pub(crate) struct HeadSplitFacts {
    pub(crate) first_is_call_or_numeric_index: bool,
    pub(crate) starts_with_member: bool,
    pub(crate) cap_member_promotion_before_call_tail: bool,
}

/// Build static head split signals once.
pub(crate) fn collect_head_split_facts(
    context: &DestackFormatContext<'_>,
    operations: &[ChainExpression],
    allow_wide_head: bool,
) -> HeadSplitFacts {
    let first_is_call_or_numeric_index = operations
        .first()
        .is_some_and(|operation| operation_is_call_or_numeric_index(context, operation));
    let starts_with_member = operations.first().is_some_and(operation_is_member);
    let has_call_like_tail = operations.iter().any(operation_is_call_like);

    let cap_member_promotion_before_call_tail =
        starts_with_member && has_call_like_tail && !allow_wide_head;

    HeadSplitFacts {
        first_is_call_or_numeric_index,
        starts_with_member,
        cap_member_promotion_before_call_tail,
    }
}

/// Return whether one operation is call-like.
pub(crate) fn operation_is_call_like(operation: &ChainExpression) -> bool {
    matches!(
        operation,
        ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
    )
}

/// Return whether one operation is a member access.
pub(crate) fn operation_is_member(operation: &ChainExpression) -> bool {
    matches!(operation, ChainExpression::Member { .. })
}

/// Return whether one operation is an optional hop.
pub(crate) fn operation_is_maybe(operation: &ChainExpression) -> bool {
    matches!(operation, ChainExpression::Maybe { .. })
}

/// Return whether one operation is a numeric index.
pub(crate) fn operation_is_numeric_index(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    matches!(operation, ChainExpression::Index { index, .. } if is_numeric_index(context, index))
}

/// Return whether one operation is call-like or numeric index.
pub(crate) fn operation_is_call_or_numeric_index(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    operation_is_call_like(operation) || operation_is_numeric_index(context, operation)
}

/// Return whether one following operation is call-like or index-like.
pub(crate) fn next_operation_is_call_or_index(next_operation: Option<&ChainExpression>) -> bool {
    matches!(
        next_operation,
        Some(
            ChainExpression::Call { .. }
                | ChainExpression::Index { .. }
                | ChainExpression::Instantiation { .. }
        )
    )
}

/// Return whether the previous operation is a direct call.
pub(crate) fn previous_operation_is_direct_call(
    operations: &[ChainExpression],
    index: usize,
) -> bool {
    operations
        .get(index.saturating_sub(1))
        .is_some_and(|operation| {
            matches!(
                operation,
                ChainExpression::Call {
                    position: PostfixPosition::Direct,
                    ..
                }
            )
        })
}

/// Return whether index-heavy member chains should keep the split.
pub(crate) fn should_keep_index_heavy_member_chain_split(
    facts: &HeadSplitFacts,
    has_index_tail: bool,
    allow_wide_head: bool,
) -> bool {
    facts.starts_with_member && has_index_tail && !allow_wide_head
}

/// Return whether member-pair promotion should stop after one promotion.
pub(crate) fn should_stop_after_member_promotion_cap(
    facts: &HeadSplitFacts,
    head_ops_count: usize,
) -> bool {
    facts.cap_member_promotion_before_call_tail && head_ops_count > 0
}

/// Return whether a single member-call pair after a call-like base may be promoted.
pub(crate) fn allow_single_member_call_pair_after_call_like_base(
    context: &DestackFormatContext<'_>,
    base_has_leading_call_like: bool,
    index: usize,
    operations: &[ChainExpression],
    next_operation: &ChainExpression,
) -> bool {
    base_has_leading_call_like
        && index == 0
        && operations.len() == 2
        && matches!(
            next_operation,
            ChainExpression::Call {
                node_id,
                dynamic_arguments,
                ..
            } if dynamic_arguments.len() == 1
                && !context.node_has_newline(*node_id)
                && !chain_node_has_non_inline_annotation(context, *node_id)
        )
}

/// Return whether a member-call pair should stop for call-like chain heads.
pub(crate) fn should_stop_for_call_like_head(
    facts: &HeadSplitFacts,
    base_has_leading_call_like: bool,
    allow_single_member_call_pair_after_call_like_base: bool,
) -> bool {
    facts.first_is_call_or_numeric_index
        || (base_has_leading_call_like && !allow_single_member_call_pair_after_call_like_base)
}

/// Return whether the current member-call pair is the only operation pair.
pub(crate) fn is_single_member_call_pair(
    index: usize,
    operations: &[ChainExpression],
    next_operation: &ChainExpression,
) -> bool {
    index == 0
        && operations.len() == 2
        && matches!(
            next_operation,
            ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
        )
}

/// Return whether a later member hop should keep this pair split.
pub(crate) fn should_stop_for_later_member_hop(
    operations: &[ChainExpression],
    index: usize,
    is_conditional_branch: bool,
) -> bool {
    let has_later_member_hop = operations.get(index + 2..).is_some_and(|tail| {
        tail.iter()
            .any(|operation| matches!(operation, ChainExpression::Member { .. }))
    });

    has_later_member_hop && !is_conditional_branch
}

/// Return whether the next call may expand while staying in the head.
pub(crate) fn next_call_can_expand(
    context: &DestackFormatContext<'_>,
    next_operation: &ChainExpression,
) -> bool {
    matches!(
        next_operation,
        ChainExpression::Call {
            node_id,
            static_arguments,
            dynamic_arguments,
            ..
        } if chain_call_can_expand_in_head(
            context,
            *node_id,
            static_arguments,
            dynamic_arguments
        )
    )
}

/// Return whether the next call is a promotable single-argument call.
pub(crate) fn next_is_promotable_single_argument_call(
    context: &DestackFormatContext<'_>,
    next_operation: &ChainExpression,
    allow_wide_head: bool,
    operations_len: usize,
) -> bool {
    matches!(
        next_operation,
        ChainExpression::Call {
            node_id,
            dynamic_arguments,
            ..
        } if dynamic_arguments.len() == 1
            && (allow_wide_head || operations_len == 2)
            && !chain_node_has_non_inline_annotation(context, *node_id)
    )
}

/// Return whether this member-call pair should remain split.
pub(crate) fn should_keep_member_call_pair_split(
    allow_wide_head: bool,
    next_call_can_expand: bool,
) -> bool {
    !allow_wide_head && next_call_can_expand
}

/// Return whether one member-call pair is too complex to promote.
pub(crate) fn should_stop_for_non_simple_member_call_pair(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
    next_operation: &ChainExpression,
    allow_single_member_call_pair_promotion: bool,
    next_call_can_expand: bool,
    next_is_promotable_single_argument_call: bool,
) -> bool {
    (!is_simple_chain_operation(context, operation) && !allow_single_member_call_pair_promotion)
        || (!is_simple_chain_operation(context, next_operation)
            && !next_call_can_expand
            && !next_is_promotable_single_argument_call
            && !allow_single_member_call_pair_promotion)
}

/// Return whether one call has no arguments.
pub(crate) fn operation_is_empty_call(operation: &ChainExpression) -> bool {
    matches!(
        operation,
        ChainExpression::Call {
            static_arguments,
            dynamic_arguments,
            ..
        } if static_arguments
            .as_ref()
            .is_none_or(|arguments| arguments.is_empty())
            && dynamic_arguments.is_empty()
    )
}

/// Return whether call-root chains should stop before an empty call tail.
pub(crate) fn should_stop_for_call_root_empty_call_tail(
    base_has_leading_call_like: bool,
    next_is_empty_call: bool,
) -> bool {
    base_has_leading_call_like && next_is_empty_call
}

/// Return whether one operation should stop because it is not simple.
pub(crate) fn should_stop_for_non_simple_operation(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    !is_simple_chain_operation(context, operation)
}

/// Return whether call-like or numeric-index heads should stop for this operation shape.
pub(crate) fn should_stop_for_call_or_numeric_head_mismatch(
    context: &DestackFormatContext<'_>,
    facts: &HeadSplitFacts,
    operation: &ChainExpression,
) -> bool {
    if !facts.first_is_call_or_numeric_index {
        return false;
    }

    !operation_is_call_like(operation) && !operation_is_numeric_index(context, operation)
}

/// Return whether member heads should stop before call-like operations.
pub(crate) fn should_stop_for_member_head_call(
    facts: &HeadSplitFacts,
    is_call: bool,
    previous_operation_is_direct_call: bool,
) -> bool {
    !facts.first_is_call_or_numeric_index && is_call && !previous_operation_is_direct_call
}

/// Return whether one chain contains any index operation.
pub(crate) fn chain_has_index_tail(operations: &[ChainExpression]) -> bool {
    operations
        .iter()
        .any(|operation| matches!(operation, ChainExpression::Index { .. }))
}

/// Return whether one operation should stop member-pair promotion.
pub(crate) fn should_stop_for_member_pair(
    context: &DestackFormatContext<'_>,
    facts: &HeadSplitFacts,
    base_has_leading_call_like: bool,
    index: usize,
    operations: &[ChainExpression],
    next_operation: &ChainExpression,
    allow_wide_head: bool,
    is_conditional_branch: bool,
    operation: &ChainExpression,
) -> bool {
    let allow_single_member_call_pair_after_call_like_base =
        allow_single_member_call_pair_after_call_like_base(
            context,
            base_has_leading_call_like,
            index,
            operations,
            next_operation,
        );
    if should_stop_for_call_like_head(
        facts,
        base_has_leading_call_like,
        allow_single_member_call_pair_after_call_like_base,
    ) {
        return true;
    }

    let is_single_member_call_pair = is_single_member_call_pair(index, operations, next_operation);
    let allow_single_member_call_pair_promotion = allow_wide_head && is_single_member_call_pair;

    if should_stop_for_later_member_hop(operations, index, is_conditional_branch) {
        return true;
    }

    let next_call_can_expand = next_call_can_expand(context, next_operation);
    let next_is_promotable_single_argument_call = next_is_promotable_single_argument_call(
        context,
        next_operation,
        allow_wide_head,
        operations.len(),
    );

    if should_keep_member_call_pair_split(allow_wide_head, next_call_can_expand) {
        return true;
    }

    if should_stop_for_non_simple_member_call_pair(
        context,
        operation,
        next_operation,
        allow_single_member_call_pair_promotion,
        next_call_can_expand,
        next_is_promotable_single_argument_call,
    ) {
        return true;
    }

    let next_is_empty_call = operation_is_empty_call(next_operation);
    should_stop_for_call_root_empty_call_tail(base_has_leading_call_like, next_is_empty_call)
}

/// Return whether one operation should stop linear head promotion.
pub(crate) fn should_stop_for_linear_operation(
    context: &DestackFormatContext<'_>,
    facts: &HeadSplitFacts,
    operation: &ChainExpression,
    previous_operation_is_direct_call: bool,
) -> bool {
    if should_stop_for_non_simple_operation(context, operation) {
        return true;
    }

    if should_stop_for_call_or_numeric_head_mismatch(context, facts, operation) {
        return true;
    }

    let is_call = operation_is_call_like(operation);
    should_stop_for_member_head_call(facts, is_call, previous_operation_is_direct_call)
}

/// Return whether one operation is member plus call-or-index pair eligible.
pub(crate) fn is_member_call_or_index_pair(
    operation: &ChainExpression,
    next_operation: Option<&ChainExpression>,
) -> bool {
    operation_is_member(operation) && next_operation_is_call_or_index(next_operation)
}

/// Split off simple head operations that should stay with the base.
pub(crate) fn split_chain_head_operations(
    context: &DestackFormatContext<'_>,
    base_has_leading_call_like: bool,
    operations: &[ChainExpression],
    allow_wide_head: bool,
    is_conditional_branch: bool,
) -> usize {
    // nothing to split when there are no operations
    if operations.is_empty() {
        return 0;
    }

    // collect static signals once
    let facts = collect_head_split_facts(context, operations, allow_wide_head);
    let has_index_tail = chain_has_index_tail(operations);
    if should_keep_index_heavy_member_chain_split(&facts, has_index_tail, allow_wide_head) {
        return 0;
    }

    // accumulate promotable simple operations
    let mut head_ops_count = 0usize;
    let mut index = 0usize;

    while index < operations.len() {
        if should_stop_after_member_promotion_cap(&facts, head_ops_count) {
            break;
        }

        let operation = &operations[index];
        if operation_is_maybe(operation) {
            break;
        }

        let next_operation = operations.get(index + 1);
        if is_member_call_or_index_pair(operation, next_operation) {
            let Some(next_operation) = next_operation else {
                break;
            };

            if should_stop_for_member_pair(
                context,
                &facts,
                base_has_leading_call_like,
                index,
                operations,
                next_operation,
                allow_wide_head,
                is_conditional_branch,
                operation,
            ) {
                break;
            }

            head_ops_count += 2;
            index += 2;
            continue;
        }

        let previous_op_is_direct_call = previous_operation_is_direct_call(operations, index);
        if should_stop_for_linear_operation(context, &facts, operation, previous_op_is_direct_call)
        {
            break;
        }

        head_ops_count += 1;
        index += 1;
    }

    head_ops_count
}

/// Return whether a chain contains trivia between adjacent chain operations.
pub(crate) fn chain_has_intervening_break_or_comment(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    // check adjacent chain nodes directly for source comments
    for adjacent in chain.windows(2) {
        let left_id = adjacent[0];
        let right_id = adjacent[1];
        if has_comment_between_expressions(context, left_id, right_id) {
            return true;
        }
    }

    chain.iter().copied().any(|expression_id| {
        member_has_intervening_break_or_comment(context, expression_id)
            || chain_has_parent_intervening_break_or_comment(context, expression_id)
    })
}

/// Return whether a chain contains comments between adjacent chain operations.
pub(crate) fn chain_has_intervening_comment(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    // check adjacent chain nodes directly for source comments
    for adjacent in chain.windows(2) {
        let left_id = adjacent[0];
        let right_id = adjacent[1];
        if has_comment_between_expressions(context, left_id, right_id) {
            return true;
        }
    }

    chain.iter().copied().any(|expression_id| {
        member_has_intervening_comment(context, expression_id)
            || chain_has_parent_intervening_comment(context, expression_id)
    })
}

/// Check if a chain node has source comments before its parent operator.
fn chain_has_parent_intervening_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent = context.tree.get(parent_id);
    let parent_uses_node_as_left = match parent {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => *left == node_id,
        _ => false,
    };
    if !parent_uses_node_as_left {
        return false;
    }

    let should_check_parent_gap = match parent {
        Expression::Member { .. } | Expression::PrivateMember { .. } => true,
        Expression::Call { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => matches!(
            context.tree.get(node_id),
            Expression::Member { .. } | Expression::PrivateMember { .. } | Expression::Path { .. }
        ),
        _ => false,
    };
    if !should_check_parent_gap {
        return false;
    }

    let node_span = context.span(node_id);
    let node_anchor_end = expression_trivia_anchor_end(context, node_id);
    let Some(parent_main_span) = context.tree.get_main_span(parent_id) else {
        return false;
    };
    if node_span.file != parent_main_span.file || parent_main_span.start <= node_anchor_end {
        return false;
    }

    let between_span = Span::new(node_span.file, node_anchor_end, parent_main_span.start);
    span_has_comment(context, between_span)
}

/// Store call-parent facts for one expression.
#[derive(Default)]
pub(crate) struct CallParentFacts {
    /// Whether the expression is a call expression.
    pub(crate) is_call_expression: bool,
    /// Whether the call callee is a member-like expression.
    pub(crate) callee_is_member_expression: bool,
    /// Whether the member receiver is parenthesized.
    pub(crate) member_receiver_is_parenthesized: bool,
    /// Whether the parenthesized receiver inner expression is await-like.
    pub(crate) member_receiver_inner_is_await_like: bool,
}

/// Store receiver-parent facts for one expression.
#[derive(Default)]
pub(crate) struct ReceiverParentFacts {
    /// Whether the direct parent is await-like with this receiver as operand.
    pub(crate) parent_is_await_on_receiver: bool,
    /// Whether the direct parent is a parenthesized wrapper over this receiver.
    pub(crate) parent_is_parenthesized_on_receiver: bool,
    /// Whether the grandparent is await-like over the parenthesized wrapper.
    pub(crate) grandparent_is_await_on_parenthesized: bool,
}

/// Store type-binary-left parent facts for one expression.
#[derive(Default)]
pub(crate) struct TypeBinaryLeftFacts {
    /// Whether the direct parent is a cast or satisfies type-binary with this node as left side.
    pub(crate) parent_is_cast_or_satisfies_left: bool,
    /// Whether the type-binary parent has non-blank annotations.
    pub(crate) parent_has_non_blank_annotation: bool,
    /// Whether the type-binary node is wrapped by parenthesized and owned by `new`.
    pub(crate) has_parenthesized_new_owner: bool,
    /// Whether the owning `new` expression has non-blank annotations.
    pub(crate) new_owner_has_non_blank_annotation: bool,
}

/// Return whether one expression is await-like.
fn expression_is_await_like(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
    matches!(
        tree.get(expression_id),
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    )
}

/// Collect call-parent facts for one expression.
pub(crate) fn collect_call_parent_facts(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> CallParentFacts {
    let mut facts = CallParentFacts::default();

    let Expression::Call { left, .. } = context.tree.get(node_id) else {
        return facts;
    };
    facts.is_call_expression = true;

    let member_receiver_id = match context.tree.get(*left) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => *left,
        _ => return facts,
    };
    facts.callee_is_member_expression = true;

    let Expression::Parenthesized { expression } = context.tree.get(member_receiver_id) else {
        return facts;
    };
    facts.member_receiver_is_parenthesized = true;
    facts.member_receiver_inner_is_await_like = expression_is_await_like(context.tree, *expression);

    facts
}

/// Collect receiver-parent facts for one receiver expression.
pub(crate) fn collect_receiver_parent_facts(
    context: &DestackFormatContext<'_>,
    receiver_id: LocalNodeId<Expression>,
) -> ReceiverParentFacts {
    let mut facts = ReceiverParentFacts::default();

    let Some((parent_id, parent_type)) = context.parent(receiver_id) else {
        return facts;
    };
    if parent_type != NodeType::Expression {
        return facts;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    if matches!(
        context.tree.get(parent_expression_id),
        Expression::Await { expression } | Expression::AwaitMaybe { expression }
            if *expression == receiver_id
    ) {
        facts.parent_is_await_on_receiver = true;
        return facts;
    }

    let Expression::Parenthesized { expression } = context.tree.get(parent_expression_id) else {
        return facts;
    };
    if *expression != receiver_id {
        return facts;
    }
    facts.parent_is_parenthesized_on_receiver = true;

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_expression_id) else {
        return facts;
    };
    if grandparent_type != NodeType::Expression {
        return facts;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    facts.grandparent_is_await_on_parenthesized = matches!(
        context.tree.get(grandparent_expression_id),
        Expression::Await { expression } | Expression::AwaitMaybe { expression }
            if *expression == parent_expression_id
    );

    facts
}

/// Collect type-binary-left parent facts for one chain tail expression.
pub(crate) fn collect_type_binary_left_facts(
    context: &DestackFormatContext<'_>,
    chain_tail: LocalNodeId<Expression>,
) -> TypeBinaryLeftFacts {
    let mut facts = TypeBinaryLeftFacts::default();

    let Some((parent_id, parent_type)) = context.parent(chain_tail) else {
        return facts;
    };
    if parent_type != NodeType::Expression {
        return facts;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::TypeBinary { left, operator, .. } = context.tree.get(parent_expression_id)
    else {
        return facts;
    };
    if *left != chain_tail {
        return facts;
    }
    if !matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) {
        return facts;
    }
    facts.parent_is_cast_or_satisfies_left = true;
    facts.parent_has_non_blank_annotation = context.has_non_blank_annotation(parent_expression_id);

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_expression_id) else {
        return facts;
    };
    if grandparent_type != NodeType::Expression {
        return facts;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    let Expression::Parenthesized { expression } = context.tree.get(grandparent_expression_id)
    else {
        return facts;
    };
    if *expression != parent_expression_id {
        return facts;
    }

    let Some((great_grandparent_id, great_grandparent_type)) =
        context.parent(grandparent_expression_id)
    else {
        return facts;
    };
    if great_grandparent_type != NodeType::Expression {
        return facts;
    }

    let great_grandparent_expression_id = LocalNodeId::<Expression>::new(great_grandparent_id);
    let Expression::New { left, .. } = context.tree.get(great_grandparent_expression_id) else {
        return facts;
    };
    if *left != grandparent_expression_id {
        return facts;
    }

    facts.has_parenthesized_new_owner = true;
    facts.new_owner_has_non_blank_annotation =
        context.has_non_blank_annotation(great_grandparent_expression_id);

    facts
}

/// Return whether call-parent facts require non-chain routing for await-wrapped member receivers.
pub(crate) fn call_has_parenthesized_await_member_receiver(facts: &CallParentFacts) -> bool {
    facts.is_call_expression
        && facts.callee_is_member_expression
        && facts.member_receiver_is_parenthesized
        && facts.member_receiver_inner_is_await_like
}

/// Return whether receiver-parent facts indicate await-like wrapping.
pub(crate) fn receiver_is_await_wrapped(facts: &ReceiverParentFacts) -> bool {
    facts.parent_is_await_on_receiver
        || (facts.parent_is_parenthesized_on_receiver
            && facts.grandparent_is_await_on_parenthesized)
}

/// Return whether type-binary-left facts require chain overflow breaking.
pub(crate) fn chain_overflows_in_type_binary_left(facts: &TypeBinaryLeftFacts) -> bool {
    facts.parent_is_cast_or_satisfies_left
        && (facts.parent_has_non_blank_annotation
            || (facts.has_parenthesized_new_owner && facts.new_owner_has_non_blank_annotation))
}

/// Store parent-context facts used by chain routing and breaking.
pub(crate) struct ChainParentFacts {
    /// The call-parent facts for the analyzed expression.
    call: CallParentFacts,
    /// The receiver-parent facts for the analyzed expression.
    receiver: ReceiverParentFacts,
    /// The type-binary-left parent facts for the analyzed expression.
    type_binary_left: TypeBinaryLeftFacts,
}

impl ChainParentFacts {
    /// Return whether a call has a member receiver wrapped in parenthesized await-like expression.
    pub(crate) fn call_has_parenthesized_await_member_receiver(&self) -> bool {
        call_has_parenthesized_await_member_receiver(&self.call)
    }

    /// Return whether one receiver sits under await-like wrapping.
    pub(crate) fn receiver_is_await_wrapped(&self) -> bool {
        receiver_is_await_wrapped(&self.receiver)
    }

    /// Return whether one chain tail overflows through cast or satisfies parent context.
    pub(crate) fn chain_overflows_in_type_binary_left(&self) -> bool {
        chain_overflows_in_type_binary_left(&self.type_binary_left)
    }
}

/// Collect parent-context facts once for one expression id.
pub(crate) fn analyze_chain_parent_facts(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> ChainParentFacts {
    ChainParentFacts {
        call: collect_call_parent_facts(context, node_id),
        receiver: collect_receiver_parent_facts(context, node_id),
        type_binary_left: collect_type_binary_left_facts(context, node_id),
    }
}

/// Return whether an expression appears inside a template literal interpolation.
pub(crate) fn expression_is_in_template_literal_interpolation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context.expression_is_in_template_literal_interpolation(expression_id)
}

/// Return whether a path root should be split into synthetic chain segments.
pub(crate) fn should_split_chain_root_path_segments(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Path { path, .. } = context.tree.get(root_id) else {
        return false;
    };
    if path.segments.len() <= 1 {
        return false;
    }

    // preserve compact callee-style heads inside argument positions
    if is_call_like_argument(context, root_id) {
        return false;
    }

    let has_optional_or_must_tail = path_chain_has_optional_or_must_tail(context, root_id);
    let has_root_line_postfix_boundary_comment = context
        .visit_annotations(root_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false);
    let has_boundary_comments = has_root_line_postfix_boundary_comment;

    // keep factory style roots merged by default, unless boundary comments need a seam
    let first_segment = context.strings.get(path.segments[0]);
    if is_factory_like_path_head(first_segment) && !has_boundary_comments {
        return false;
    }
    if first_segment == "this" && !has_optional_or_must_tail && !has_boundary_comments {
        return false;
    }

    // conditional branches keep compact path heads
    if expression_is_in_conditional_branch(context, root_id) {
        return false;
    }

    true
}

/// Return whether a path chain has optional or must tail operators.
pub(crate) fn path_chain_has_optional_or_must_tail(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> bool {
    let mut current = root_id;

    while let Some((parent_id, parent_type)) = context.parent(current) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        let parent = context.tree.get(parent_id);
        let parent_uses_left = match parent {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => *left == current,
            _ => false,
        };
        if !parent_uses_left {
            break;
        }

        if matches!(
            parent,
            Expression::Maybe { .. }
                | Expression::Must { .. }
                | Expression::Call {
                    position: PostfixPosition::Indirect,
                    ..
                }
                | Expression::Index {
                    position: PostfixPosition::Indirect,
                    ..
                }
        ) {
            return true;
        }

        current = parent_id;
    }

    false
}

/// Return whether a path head looks like a factory identifier.
pub(crate) fn is_factory_like_path_head(name: &str) -> bool {
    let mut bytes = name.bytes();
    match bytes.next() {
        Some(b'_' | b'$') => bytes.all(|byte| matches!(byte, b'_' | b'$')),
        Some(byte) => byte.is_ascii_uppercase(),
        None => false,
    }
}

/// Return whether an expression is inside a ternary branch.
pub(crate) fn expression_is_in_conditional_branch(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context.any_ancestor(expression_id, |ancestor_id, node_type| {
        node_type == NodeType::Expression
            && matches!(
                context
                    .tree
                    .get(LocalNodeId::<Expression>::new(ancestor_id)),
                Expression::If {
                    kind: IfKind::Ternary,
                    ..
                }
            )
    })
}

/// Check whether an assignment chain ends in a nested lambda expression.
pub(crate) fn is_assignment_chain_tail_lambda(
    context: &DestackFormatContext<'_>,
    assignment_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    // only assignment-like rhs positions participate in assignment chains
    if assignment_like_parent(context, assignment_id).is_none() {
        return false;
    }

    // intermediate assignments are not chain tails
    if matches!(context.tree.get(right_id), Expression::Assign { .. }) {
        return false;
    }

    is_nested_lambda_expression(context, right_id)
}
