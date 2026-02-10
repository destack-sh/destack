use super::*;

/// Split off simple head operations that should stay with the base.
pub(crate) fn split_chain_head_operations(
    context: &DestackFormatContext<'_>,
    base_len: usize,
    base_has_leading_call_like: bool,
    operations: &[ChainExpression],
    remaining_width: Option<usize>,
    allow_wide_head: bool,
) -> usize {
    // nothing to split when there are no operations
    if operations.is_empty() {
        return 0;
    }

    // keep promoted head operations within the current inline budget
    let line_width = usize::from(context.options.line_width);
    let max_head_len = remaining_width.unwrap_or(line_width);

    // detect whether the chain starts with calls or numeric indexes
    let first_is_call_or_numeric_index = match operations.first() {
        Some(ChainExpression::Call { .. }) => true,
        Some(ChainExpression::Instantiation { .. }) => true,
        Some(ChainExpression::Index { index, .. }) => is_numeric_index(context, index),
        _ => false,
    };
    let starts_with_member = matches!(operations.first(), Some(ChainExpression::Member { .. }));
    let has_call_like_tail = operations.iter().any(|operation| {
        matches!(
            operation,
            ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
        )
    });
    let cap_member_promotion_before_call_tail =
        starts_with_member && has_call_like_tail && remaining_width.is_none();

    // accumulate simple operations while within the promotion limits
    let mut head_len = base_len;
    let mut head_ops_count = 0usize;
    let mut index = 0usize;

    while index < operations.len() {
        // keep member-leading fluent call chains from over-promoting the head
        if cap_member_promotion_before_call_tail && head_ops_count > 0 {
            break;
        }

        let operation = &operations[index];
        if matches!(operation, ChainExpression::Maybe { .. }) {
            break;
        }

        // avoid splitting a member from its immediate call or index
        let next_operation = operations.get(index + 1);
        let next_is_call_or_index = matches!(
            next_operation,
            Some(
                ChainExpression::Call { .. }
                    | ChainExpression::Index { .. }
                    | ChainExpression::Instantiation { .. }
            )
        );

        if matches!(operation, ChainExpression::Member { .. }) && next_is_call_or_index {
            // only promote the pair when both operations are simple
            let Some(next_operation) = next_operation else {
                break;
            };

            // when a chain has more member hops after a member + call pair:
            // keep fluent chains one hop per line
            let has_later_member_hop = operations.get(index + 2..).is_some_and(|tail| {
                tail.iter()
                    .any(|op| matches!(op, ChainExpression::Member { .. }))
            });
            if has_later_member_hop {
                break;
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

            if !is_simple_chain_operation(context, operation)
                || (!is_simple_chain_operation(context, next_operation)
                    && !next_call_can_expand
                    && !next_is_promotable_single_argument_call)
            {
                break;
            }

            // keep fluent `foo().bar().baz(...)` call ladders one call per line
            // when the chain already starts with a direct call
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
            if base_has_leading_call_like && next_is_empty_call {
                break;
            }

            // keep call-start chains restricted to call-like operations
            let next_is_call_like = matches!(
                next_operation,
                ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
            );
            let next_is_numeric_index = matches!(
                next_operation,
                ChainExpression::Index { index, .. } if is_numeric_index(context, index)
            );
            if first_is_call_or_numeric_index && !(next_is_call_like || next_is_numeric_index) {
                break;
            }

            // stop if promoting the pair would make the head too long
            let member_len = chain_operation_len(context, operation);
            let next_len = chain_head_operation_len(context, next_operation);
            let combined_len = head_len.saturating_add(member_len).saturating_add(next_len);
            if combined_len > max_head_len
                && !(next_call_can_expand && combined_len <= line_width)
                && !(next_is_promotable_single_argument_call && combined_len <= line_width)
            {
                break;
            }

            head_len = combined_len;
            head_ops_count += 2;
            index += 2;
            continue;
        }

        let previous_op_is_direct_call =
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
                });
        let allow_non_simple_direct_curried_tail = matches!(
            operation,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            }
        ) && previous_op_is_direct_call;
        let allow_non_simple_direct_curried_tail_after_head_call = matches!(
            operation,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            }
        ) && base_has_leading_call_like
            && head_ops_count == 0;

        // only promote simple operations
        if !is_simple_chain_operation(context, operation)
            && !allow_non_simple_direct_curried_tail
            && !allow_non_simple_direct_curried_tail_after_head_call
        {
            break;
        }

        let is_call = matches!(
            operation,
            ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
        );
        let is_numeric_index_op = matches!(
            operation,
            ChainExpression::Index { index, .. } if is_numeric_index(context, index)
        );

        // when the chain starts with calls, keep only call-like head operations
        if first_is_call_or_numeric_index && !(is_call || is_numeric_index_op) {
            break;
        }

        // when the chain starts with members, stop before the first call
        // allow direct call tails after a promoted call: `foo(...)(...)`
        if !first_is_call_or_numeric_index && is_call && !previous_op_is_direct_call {
            break;
        }

        // stop if promoting this operation would make the head too long
        let operation_len = chain_head_operation_len(context, operation);
        let next_len = head_len.saturating_add(operation_len);
        if next_len > max_head_len {
            let allow_direct_curried_tail = matches!(
                operation,
                ChainExpression::Call {
                    position: PostfixPosition::Direct,
                    ..
                }
            ) && (previous_op_is_direct_call
                || allow_non_simple_direct_curried_tail_after_head_call)
                && next_len <= line_width;
            if !allow_direct_curried_tail {
                break;
            }
        }

        head_len = next_len;
        head_ops_count += 1;
        index += 1;
    }

    head_ops_count
}

/// Summarize the complexity of a call within a chain.
pub(crate) struct ChainCallSummary {
    pub(crate) has_multiline_argument: bool,
}

/// Collect break-relevant signals for one chain.
pub(crate) struct ChainBreakAnalysis {
    pub(crate) should_break: bool,
    pub(crate) call_summaries: Vec<ChainCallSummary>,
    pub(crate) has_chain_intervening_trivia: bool,
    pub(crate) has_path_tail_deferred_empty_call_boundary_comment: bool,
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
        summaries.push(ChainCallSummary {
            has_multiline_argument,
        });
    }

    summaries
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
            has_path_tail_deferred_empty_call_boundary_comment: false,
        };
    }

    let line_width = usize::from(context.options.line_width);
    let chain_root = chain[0];
    let chain_tail = chain[chain.len() - 1];
    let chain_head = chain_head_id(context.tree, chain_root);
    let call_summaries = summarize_chain_calls(context, chain);
    let has_chain_intervening_trivia = chain_has_intervening_break_or_comment(context, chain);
    let has_deferred_empty_call_boundary_comment = chain.iter().copied().any(|expression_id| {
        expression_is_in_deferred_empty_call_boundary_chain(context, expression_id)
    });
    let root_has_path_tail_segments = matches!(
        context.tree.get(chain_root),
        Expression::Path { path, .. } if path.segments.len() > 1
    );
    let has_path_tail_deferred_empty_call_boundary_comment =
        root_has_path_tail_segments && has_deferred_empty_call_boundary_comment;
    let has_optional_tail = chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Maybe { .. }
                | Expression::Call {
                    position: PostfixPosition::Indirect,
                    ..
                }
        )
    });
    let has_member_access = chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
    });
    let has_chain_annotations = chain
        .iter()
        .copied()
        .any(|expression_id| chain_node_has_breaking_annotation(context, expression_id))
        || chain_node_has_breaking_annotation(context, chain_head);
    let should_break_for_annotation_or_trivia =
        has_chain_annotations || (has_chain_intervening_trivia && has_member_access);
    if should_break_for_annotation_or_trivia {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
            has_path_tail_deferred_empty_call_boundary_comment,
        };
    }

    let should_break_for_path_tail_comment =
        has_path_tail_deferred_empty_call_boundary_comment && has_member_access;
    if should_break_for_path_tail_comment {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
            has_path_tail_deferred_empty_call_boundary_comment,
        };
    }

    let should_break_for_path_optional_tail =
        has_chain_intervening_trivia && root_has_path_tail_segments && has_optional_tail;
    if should_break_for_path_optional_tail {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
            has_path_tail_deferred_empty_call_boundary_comment,
        };
    }

    // chains with no calls stay inline unless they overflow
    if call_summaries.is_empty() {
        return ChainBreakAnalysis {
            should_break: false,
            call_summaries,
            has_chain_intervening_trivia,
            has_path_tail_deferred_empty_call_boundary_comment,
        };
    }

    let calls_count = call_summaries.len();
    let has_multiline_call = call_summaries
        .iter()
        .any(|summary| summary.has_multiline_argument);
    let chain_is_in_conditional_branch = expression_is_in_conditional_branch(context, chain_tail)
        || expression_is_in_conditional_branch(context, chain_root);
    if calls_count > 1 && has_multiline_call && !chain_is_in_conditional_branch {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
            has_path_tail_deferred_empty_call_boundary_comment,
        };
    }

    // single-call chains should prefer the regular inline fit decision
    if calls_count == 1 {
        return ChainBreakAnalysis {
            should_break: false,
            call_summaries,
            has_chain_intervening_trivia,
            has_path_tail_deferred_empty_call_boundary_comment,
        };
    }

    let in_template_literal_interpolation = chain.iter().copied().any(|expression_id| {
        expression_is_in_template_literal_interpolation(context, expression_id)
    });
    let available_width =
        if is_call_like_argument(context, chain_tail) || in_template_literal_interpolation {
            line_width
        } else {
            assignment_like_remaining_width(context, chain_root).unwrap_or(line_width)
        };
    let should_break = chain_overflows_in_type_binary_left(context, chain_tail, available_width);

    ChainBreakAnalysis {
        should_break,
        call_summaries,
        has_chain_intervening_trivia,
        has_path_tail_deferred_empty_call_boundary_comment,
    }
}

/// Check whether a chain node has an annotation that should force breaking.
pub(crate) fn chain_node_has_breaking_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .with_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.tree.get::<Annotation>(*annotation_id);
                let position = annotation.position();
                if is_deferred_empty_call_boundary_annotation(
                    context,
                    node_id,
                    *annotation_id,
                    position,
                ) {
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
    context
        .with_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.tree.get::<Annotation>(*annotation_id);
                let position = annotation.position();
                if is_deferred_empty_call_boundary_annotation(
                    context,
                    node_id,
                    *annotation_id,
                    position,
                ) {
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
        .with_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.tree.get::<Annotation>(*annotation_id).position(),
                    AnnotationPosition::BlockPrefix
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether a chain should break because its cast or satisfies parent overflows.
pub(crate) fn chain_overflows_in_type_binary_left(
    context: &DestackFormatContext<'_>,
    chain_tail: LocalNodeId<Expression>,
    available_width: usize,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(chain_tail) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::TypeBinary { left, operator, .. } = context.tree.get(parent_id) else {
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

    if expression_source_len(context, parent_id) > available_width {
        return true;
    }

    let Some((grandparent_id, grandparent_type)) = context.get_parent(parent_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_id = LocalNodeId::<Expression>::new(grandparent_id);
    let Expression::Parenthesized { expression } = context.tree.get(grandparent_id) else {
        return false;
    };
    if *expression != parent_id {
        return false;
    }

    let Some((great_grandparent_id, great_grandparent_type)) = context.get_parent(grandparent_id)
    else {
        return false;
    };
    if great_grandparent_type != NodeType::Expression {
        return false;
    }

    let great_grandparent_id = LocalNodeId::<Expression>::new(great_grandparent_id);
    let Expression::New { left, .. } = context.tree.get(great_grandparent_id) else {
        return false;
    };
    if *left != grandparent_id {
        return false;
    }

    expression_source_len(context, great_grandparent_id) > available_width
}

/// Return whether an expression appears inside a template literal interpolation.
pub(crate) fn expression_is_in_template_literal_interpolation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context.expression_is_in_template_literal_interpolation(expression_id)
}

/// Decide whether a chain should proactively break across lines.
pub(crate) fn should_break_chain(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    analyze_chain_break(context, chain).should_break
}

/// Return whether a chain contains trivia between adjacent chain operations.
pub(crate) fn chain_has_intervening_break_or_comment(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    // check adjacent chain nodes directly for source trivia
    for adjacent in chain.windows(2) {
        let left_id = adjacent[0];
        let right_id = adjacent[1];
        if has_newline_between_expressions(context, left_id, right_id)
            || has_comment_between_expressions(context, left_id, right_id)
        {
            return true;
        }
    }

    chain.iter().copied().any(|expression_id| {
        let expression = context.tree.get(expression_id);
        let is_member_expression = matches!(
            expression,
            Expression::Member { .. } | Expression::PrivateMember { .. }
        );
        if expression_is_in_deferred_empty_call_boundary_chain(context, expression_id)
            && !is_member_expression
        {
            return false;
        }

        member_has_intervening_break_or_comment(context, expression_id)
            || chain_has_parent_intervening_break_or_comment(context, expression_id)
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
    let has_deferred_boundary_comments =
        !path_deferred_boundary_line_comments(context, root_id, path.segments.len()).is_empty();

    // preserve original path-root ownership for annotated roots
    if context.has_annotation(root_id)
        && !has_optional_or_must_tail
        && !has_deferred_boundary_comments
    {
        return false;
    }

    // keep factory style roots merged by default
    let first_segment = context.strings.get(path.segments[0]);
    if is_factory_like_path_head(first_segment) {
        return false;
    }
    if first_segment == "this" && !has_optional_or_must_tail {
        return false;
    }

    // conditional branches read better with a compact head
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

    while let Some((parent_id, parent_type)) = context.get_parent(current) {
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
