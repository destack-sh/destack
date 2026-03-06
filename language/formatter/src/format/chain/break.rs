use crate::format::call::call_arguments_force_expand_for_chain;
use crate::format::chain::{
    Annotation, AnnotationPosition, ChainExpression, DestackFormatContext, Expression, IfKind,
    LocalNodeId, NodeTree, NodeType, PostfixPosition, Span, TokenType, argument_forces_multiline,
    argument_is_function_expression, argument_is_inline_closure_cast_object,
    assignment_like_parent, chain_call_can_expand_in_head,
    chain_has_parent_intervening_break_or_comment, chain_head_id, chain_parent_operator_start,
    expression_trivia_anchor_end, has_comment_between_expressions, is_call_like_argument,
    is_chain_expression, is_nested_lambda_expression, is_numeric_index, is_simple_chain_argument,
    is_simple_chain_operation, is_simple_chain_static_arguments, member_has_intervening_comment,
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
    let has_chain_root_annotations = chain
        .first()
        .is_some_and(|expression_id| chain_node_has_breaking_annotation(context, *expression_id));

    // root prefix annotations are statement-level concerns:
    // root non-prefix annotation handling stays in `chain_head_has_non_prefix_breaking_annotation`
    let has_chain_node_annotations = chain
        .iter()
        .copied()
        .skip(1)
        .any(|expression_id| chain_node_has_breaking_annotation(context, expression_id));
    let chain_head_has_non_prefix_breaking_annotation =
        chain_head_has_non_prefix_breaking_annotation(context, chain_head);

    has_chain_root_annotations
        || has_chain_node_annotations
        || chain_head_has_non_prefix_breaking_annotation
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
    let has_optional_call_boundary_trivia = chain_has_optional_call_boundary_trivia(context, chain);
    let has_optional_tail = chain_has_optional_tail(context, chain);
    let has_member_access = chain_has_member_access(context, chain);
    let has_chain_annotations = chain_has_breaking_annotations(context, chain, chain_head);
    let has_direct_curried_call_pair = chain_has_direct_curried_call_pair(context, chain);
    let has_direct_call_with_inline_closure_cast_object_argument =
        chain_has_direct_call_with_inline_closure_cast_object_argument(context, chain);
    let has_nonhead_non_simple_call_argument = call_summaries
        .iter()
        .skip(1)
        .any(|summary| summary.has_non_simple_argument);
    let has_nonhead_call_complexity = has_nonhead_non_simple_call_argument;
    let head_call_requires_expanded_arguments = chain
        .iter()
        .copied()
        .find_map(|expression_id| {
            let Expression::Call {
                dynamic_arguments, ..
            } = context.tree.get(expression_id)
            else {
                return None;
            };

            Some(call_arguments_force_expand_for_chain(
                context,
                expression_id,
                dynamic_arguments,
            ))
        })
        .unwrap_or(false);
    let chain_has_following_operation = chain.len() > 1;

    let has_call_summaries = !call_summaries.is_empty();
    let has_multiple_call_summaries = call_summaries.len() > 1;
    let should_break =
            // annotation and comment seams that always force expansion
        has_chain_annotations
            || (has_chain_intervening_comment && has_member_access)
            || has_optional_call_boundary_trivia
            || has_direct_call_with_inline_closure_cast_object_argument
            // intervening trivia around optional and curried tails
            || (has_chain_intervening_trivia
                && (has_optional_tail || has_direct_curried_call_pair))
            // call-only rules
            || (has_call_summaries
                && (has_nonhead_call_complexity
                    || (chain_has_following_operation && head_call_requires_expanded_arguments)
                    || chain_tail_parent_call_requires_chain_break(context, chain_tail)
                    || (has_multiple_call_summaries
                        && chain_overflows_in_type_binary_left(context, chain_tail))));

    ChainBreakAnalysis {
        should_break,
        call_summaries,
        has_chain_intervening_trivia,
    }
}

/// Return whether one optional call seam has boundary trivia before its operator.
fn chain_has_optional_call_boundary_trivia(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.windows(2).any(|pair| {
        let left = pair[0];
        let right = pair[1];

        let right_is_optional_call = matches!(
            context.tree.get(right),
            Expression::Call {
                position: PostfixPosition::Indirect,
                ..
            }
        );
        if !right_is_optional_call {
            return false;
        }

        chain_has_parent_intervening_break_or_comment(context, left)
    })
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
        return context.annotation_next_token_is_on_same_line(annotation_id);
    }

    if position == AnnotationPosition::LinePostfix
        && context.annotation_next_non_whitespace_token_type(annotation_id)
            == Some(TokenType::Maybe)
    {
        return true;
    }

    context.annotation_next_token_is_on_same_line(annotation_id)
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

/// Return whether member-pair promotion should stop after one promotion.
pub(crate) fn should_stop_after_member_promotion_cap(
    cap_member_promotion_before_call_tail: bool,
    head_ops_count: usize,
) -> bool {
    cap_member_promotion_before_call_tail && head_ops_count > 0
}

/// Return whether index-heavy member chains should keep the split.
pub(crate) fn should_keep_index_heavy_member_chain_split(
    starts_with_member: bool,
    has_index_tail: bool,
    has_call_like_tail: bool,
    allow_wide_head: bool,
) -> bool {
    starts_with_member && has_index_tail && !has_call_like_tail && !allow_wide_head
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
    first_is_call_or_numeric_index: bool,
    base_has_leading_call_like: bool,
    index: usize,
    operations: &[ChainExpression],
    next_operation: &ChainExpression,
    allow_wide_head: bool,
    is_conditional_branch: bool,
    operation: &ChainExpression,
) -> bool {
    let is_single_member_call_pair = index == 0
        && operations.len() == 2
        && matches!(
            next_operation,
            ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
        );
    let allow_single_member_call_pair_promotion =
        allow_wide_head && !is_conditional_branch && is_single_member_call_pair;
    if first_is_call_or_numeric_index
        || (base_has_leading_call_like && !allow_single_member_call_pair_promotion)
    {
        return true;
    }

    let next_is_empty_call = matches!(
        next_operation,
        ChainExpression::Call {
            static_arguments,
            dynamic_arguments,
            ..
        } if static_arguments
            .as_ref()
            .is_none_or(|arguments| arguments.is_empty())
            && dynamic_arguments.is_empty()
    );

    let has_later_member_hop = operations.get(index + 2..).is_some_and(|tail| {
        tail.iter()
            .any(|operation| matches!(operation, ChainExpression::Member { .. }))
    });
    if has_later_member_hop && !is_conditional_branch && !next_is_empty_call {
        return true;
    }

    let next_call_can_expand = matches!(
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
    );
    let next_is_promotable_single_argument_call = matches!(
        next_operation,
        ChainExpression::Call {
            node_id,
            dynamic_arguments,
            ..
        } if dynamic_arguments.len() == 1
            && (allow_wide_head || operations.len() == 2)
            && !chain_node_has_non_inline_annotation(context, *node_id)
    );

    if !allow_wide_head && next_call_can_expand {
        return true;
    }

    if (!is_simple_chain_operation(context, operation) && !allow_single_member_call_pair_promotion)
        || (!is_simple_chain_operation(context, next_operation)
            && !next_call_can_expand
            && !next_is_promotable_single_argument_call
            && !allow_single_member_call_pair_promotion)
    {
        return true;
    }

    base_has_leading_call_like && next_is_empty_call
}

/// Return whether one operation should stop linear head promotion.
pub(crate) fn should_stop_for_linear_operation(
    context: &DestackFormatContext<'_>,
    first_is_call_or_numeric_index: bool,
    operation: &ChainExpression,
    previous_operation_is_direct_call: bool,
) -> bool {
    if !is_simple_chain_operation(context, operation) {
        return true;
    }

    if first_is_call_or_numeric_index
        && !operation_is_call_like(operation)
        && !operation_is_numeric_index(context, operation)
    {
        return true;
    }

    let is_call = operation_is_call_like(operation);
    let is_empty_call = matches!(
        operation,
        ChainExpression::Call {
            static_arguments,
            dynamic_arguments,
            ..
        } if static_arguments
            .as_ref()
            .is_none_or(|arguments| arguments.is_empty())
            && dynamic_arguments.is_empty()
    );

    !first_is_call_or_numeric_index
        && is_call
        && !previous_operation_is_direct_call
        && !is_empty_call
}

/// Split off simple head operations that should stay with the base.
pub(crate) fn split_chain_head_operations(
    context: &DestackFormatContext<'_>,
    base_has_leading_call_like: bool,
    operations: &[ChainExpression],
    allow_wide_head: bool,
    is_conditional_branch: bool,
    root_is_parenthesized: bool,
) -> usize {
    // nothing to split when there are no operations
    if operations.is_empty() {
        return 0;
    }

    // collect static signals once
    let first_is_call_or_numeric_index = operations
        .first()
        .is_some_and(|operation| operation_is_call_or_numeric_index(context, operation));
    let starts_with_member = operations.first().is_some_and(operation_is_member);
    let has_call_like_tail = operations.iter().any(operation_is_call_like);
    let cap_member_promotion_before_call_tail =
        starts_with_member && has_call_like_tail && !allow_wide_head;
    let has_index_tail = chain_has_index_tail(operations);

    // preserve indexed-member split layout unless parenthesized roots need compact cast chains
    if should_keep_index_heavy_member_chain_split(
        starts_with_member,
        has_index_tail,
        has_call_like_tail,
        allow_wide_head,
    ) && !root_is_parenthesized
    {
        return 0;
    }

    // keep `obj.items[0]` style heads together before call tails
    let starts_with_simple_member_then_numeric_index = operations.len() >= 2
        && operation_is_member(&operations[0])
        && operation_is_numeric_index(context, &operations[1])
        && is_simple_chain_operation(context, &operations[0])
        && is_simple_chain_operation(context, &operations[1]);

    // accumulate promotable simple operations
    let mut head_ops_count = if starts_with_simple_member_then_numeric_index {
        2
    } else {
        0
    };
    let mut index = head_ops_count;

    while index < operations.len() {
        if should_stop_after_member_promotion_cap(
            cap_member_promotion_before_call_tail,
            head_ops_count,
        ) {
            break;
        }

        let operation = &operations[index];
        if operation_is_maybe(operation) {
            break;
        }

        let next_operation = operations.get(index + 1);
        if operation_is_member(operation)
            && matches!(
                next_operation,
                Some(
                    ChainExpression::Call { .. }
                        | ChainExpression::Index { .. }
                        | ChainExpression::Instantiation { .. }
                )
            )
        {
            let Some(next_operation) = next_operation else {
                break;
            };

            if should_stop_for_member_pair(
                context,
                first_is_call_or_numeric_index,
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

            // keep `member()?.` tails split so optional chains can break before the call pair
            if operations.get(index + 2).is_some_and(operation_is_maybe) {
                break;
            }

            head_ops_count += 2;
            index += 2;
            continue;
        }

        let previous_op_is_direct_call =
            operations
                .get(index.saturating_sub(1))
                .is_some_and(|previous_operation| {
                    matches!(
                        previous_operation,
                        ChainExpression::Call {
                            position: PostfixPosition::Direct,
                            ..
                        }
                    )
                });
        if should_stop_for_linear_operation(
            context,
            first_is_call_or_numeric_index,
            operation,
            previous_op_is_direct_call,
        ) {
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
    chain_has_intervening_comment(context, chain)
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
    let Some(parent_operator_start) = chain_parent_operator_start(context, node_id, parent_id)
    else {
        return false;
    };
    if parent_operator_start <= node_anchor_end {
        return false;
    }

    let between_span = Span::new(node_span.file, node_anchor_end, parent_operator_start);
    context.has_comment(between_span)
}

/// Return whether one expression is await-like.
fn expression_is_await_like(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
    matches!(
        tree.get(expression_id),
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    )
}

/// Return whether a call has a member receiver wrapped in parenthesized await-like expression.
pub(crate) fn call_has_parenthesized_await_member_receiver(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = context.tree.get(node_id) else {
        return false;
    };

    let member_receiver_id = match context.tree.get(*left) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => *left,
        _ => return false,
    };

    let Expression::Parenthesized { expression } = context.tree.get(member_receiver_id) else {
        return false;
    };
    expression_is_await_like(context.tree, *expression)
}

/// Return whether one receiver sits under await-like wrapping.
pub(crate) fn receiver_is_await_wrapped(
    context: &DestackFormatContext<'_>,
    receiver_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(receiver_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    if matches!(
        context.tree.get(parent_expression_id),
        Expression::Await { expression } | Expression::AwaitMaybe { expression }
            if *expression == receiver_id
    ) {
        return true;
    }

    let Expression::Parenthesized { expression } = context.tree.get(parent_expression_id) else {
        return false;
    };
    if *expression != receiver_id {
        return false;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_expression_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    matches!(
        context.tree.get(grandparent_expression_id),
        Expression::Await { expression } | Expression::AwaitMaybe { expression }
            if *expression == parent_expression_id
    )
}

/// Return whether one chain tail overflows through cast or satisfies parent context.
pub(crate) fn chain_overflows_in_type_binary_left(
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
    let Expression::TypeBinary { left, operator, .. } = context.tree.get(parent_expression_id)
    else {
        return false;
    };
    if *left != chain_tail {
        return false;
    }
    if !matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) {
        return false;
    }

    if context.has_non_blank_annotation(parent_expression_id) {
        return true;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_expression_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    let Expression::Parenthesized { expression } = context.tree.get(grandparent_expression_id)
    else {
        return false;
    };
    if *expression != parent_expression_id {
        return false;
    }

    let Some((great_grandparent_id, great_grandparent_type)) =
        context.parent(grandparent_expression_id)
    else {
        return false;
    };
    if great_grandparent_type != NodeType::Expression {
        return false;
    }

    let great_grandparent_expression_id = LocalNodeId::<Expression>::new(great_grandparent_id);
    let Expression::New { left, .. } = context.tree.get(great_grandparent_expression_id) else {
        return false;
    };
    if *left != grandparent_expression_id {
        return false;
    }

    context.has_non_blank_annotation(great_grandparent_expression_id)
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
