use super::*;
use smallvec::smallvec;

/// Return whether an expression has a line postfix boundary comment annotation.
pub(super) fn expression_has_line_postfix_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .with_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.get_annotation(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether a chain call has exactly one template literal argument.
fn chain_call_has_single_template_literal_argument(
    context: &DestackFormatContext<'_>,
    op: &ChainExpression,
) -> bool {
    let ChainExpression::Call {
        dynamic_arguments, ..
    } = op
    else {
        return false;
    };

    dynamic_arguments.len() == 1 && argument_is_template_literal(context, dynamic_arguments[0])
}

/// Group chain operations into the segments that should share lines.
pub(super) fn group_chain_expression_lines(
    context: &DestackFormatContext<'_>,
    operations: Vec<ChainExpression>,
) -> Vec<SmallVec<[ChainExpression; 2]>> {
    let mut lines = Vec::new();
    let mut iter = operations.into_iter().peekable();
    while let Some(op) = iter.next() {
        let mut line = smallvec![op.clone()];
        match op {
            ChainExpression::Maybe { .. } => {
                extend_maybe_line(context, &mut iter, &mut line);
            }
            ChainExpression::Member { .. } => {
                extend_member_line(context, &mut iter, &mut line);
            }
            ChainExpression::Index { .. }
            | ChainExpression::Call { .. }
            | ChainExpression::Instantiation { .. } => {
                extend_call_like_line(context, &mut iter, &mut line);
            }
            ChainExpression::Must { .. } => {
                extend_must_line(&mut iter, &mut line);
            }
        }
        lines.push(line);
    }

    merge_direct_index_lines(context, lines)
}

type ChainOperationIter = std::iter::Peekable<std::vec::IntoIter<ChainExpression>>;

/// Return whether a chain line starts with a mergeable direct index operation.
fn line_starts_with_mergeable_direct_index(
    context: &DestackFormatContext<'_>,
    line: &[ChainExpression],
) -> bool {
    let Some(ChainExpression::Index {
        node_id,
        position: PostfixPosition::Direct,
        ..
    }) = line.first()
    else {
        return false;
    };

    !chain_node_has_non_inline_annotation(context, *node_id)
}

/// Merge direct index chain lines into the previous line.
fn merge_direct_index_lines(
    context: &DestackFormatContext<'_>,
    lines: Vec<SmallVec<[ChainExpression; 2]>>,
) -> Vec<SmallVec<[ChainExpression; 2]>> {
    let mut merged_lines: Vec<SmallVec<[ChainExpression; 2]>> = Vec::with_capacity(lines.len());

    for line in lines {
        if line_starts_with_mergeable_direct_index(context, &line)
            && let Some(previous_line) = merged_lines.last_mut()
        {
            previous_line.extend(line);
            continue;
        }

        merged_lines.push(line);
    }

    merged_lines
}

/// Extend a line that starts with a maybe chain operation.
fn extend_maybe_line(
    context: &DestackFormatContext<'_>,
    iter: &mut ChainOperationIter,
    line: &mut SmallVec<[ChainExpression; 2]>,
) {
    // member based maybe tails
    if matches!(iter.peek(), Some(ChainExpression::Member { .. })) {
        push_next_chain_operation(iter, line);
        if matches!(iter.peek(), Some(ChainExpression::Must { .. })) {
            push_next_chain_operation(iter, line);
        }
        if matches!(
            iter.peek(),
            Some(
                ChainExpression::Index { .. }
                    | ChainExpression::Call { .. }
                    | ChainExpression::Instantiation { .. }
            )
        ) {
            push_next_chain_operation(iter, line);
        }

        // keep short member tails with optional call chains
        let mut merged_member_count = 0usize;
        while merged_member_count < 2 {
            let Some(ChainExpression::Member { node_id, .. }) = iter.peek() else {
                break;
            };
            if chain_node_has_non_inline_annotation(context, *node_id) {
                break;
            }
            push_next_chain_operation(iter, line);
            merged_member_count += 1;
        }

        return;
    }

    // index or call maybe tails
    if matches!(
        iter.peek(),
        Some(
            ChainExpression::Index { .. }
                | ChainExpression::Call { .. }
                | ChainExpression::Instantiation { .. }
        )
    ) {
        push_next_chain_operation(iter, line);
        if matches!(iter.peek(), Some(ChainExpression::Must { .. })) {
            push_next_chain_operation(iter, line);
        }
    }
}

/// Extend a line that starts with a member chain operation.
fn extend_member_line(
    context: &DestackFormatContext<'_>,
    iter: &mut ChainOperationIter,
    line: &mut SmallVec<[ChainExpression; 2]>,
) {
    // attach immediate must and call like operations
    if matches!(iter.peek(), Some(ChainExpression::Must { .. })) {
        push_next_chain_operation(iter, line);
    }
    if matches!(
        iter.peek(),
        Some(
            ChainExpression::Call { .. }
                | ChainExpression::Index { .. }
                | ChainExpression::Instantiation { .. }
        )
    ) {
        push_next_chain_operation(iter, line);
    }

    // keep direct curried calls attached
    while let Some(ChainExpression::Call {
        node_id,
        position: PostfixPosition::Direct,
        ..
    }) = iter.peek()
    {
        if chain_node_has_non_inline_annotation(context, *node_id) {
            break;
        }
        push_next_chain_operation(iter, line);
    }

    // merge member runs that end in a call operation
    if should_merge_member_run_with_call(context, line, iter) {
        while matches!(iter.peek(), Some(ChainExpression::Member { .. })) {
            push_next_chain_operation(iter, line);
        }
        if matches!(iter.peek(), Some(ChainExpression::Must { .. })) {
            push_next_chain_operation(iter, line);
        }
        if matches!(iter.peek(), Some(ChainExpression::Call { .. })) {
            push_next_chain_operation(iter, line);
        }
    }

    // keep short member tails together before terminal call like operations
    let should_merge_member_tail = line
        .last()
        .is_some_and(|op| chain_call_has_single_template_literal_argument(context, op));
    if !should_merge_member_tail {
        return;
    }

    let mut merged_member_count = 0usize;
    while merged_member_count < 2 && matches!(iter.peek(), Some(ChainExpression::Member { .. })) {
        push_next_chain_operation(iter, line);
        merged_member_count += 1;
    }
    if matches!(iter.peek(), Some(ChainExpression::Must { .. })) {
        push_next_chain_operation(iter, line);
    }
    if matches!(
        iter.peek(),
        Some(
            ChainExpression::Call { .. }
                | ChainExpression::Index { .. }
                | ChainExpression::Instantiation { .. }
        )
    ) {
        push_next_chain_operation(iter, line);
    }
}

/// Extend a line that starts with an index, call, or instantiation operation.
fn extend_call_like_line(
    context: &DestackFormatContext<'_>,
    iter: &mut ChainOperationIter,
    line: &mut SmallVec<[ChainExpression; 2]>,
) {
    // attach immediate must operation
    if matches!(iter.peek(), Some(ChainExpression::Must { .. })) {
        push_next_chain_operation(iter, line);
    }

    // keep direct curried calls attached
    while let Some(ChainExpression::Call {
        node_id,
        position: PostfixPosition::Direct,
        ..
    }) = iter.peek()
    {
        if chain_node_has_non_inline_annotation(context, *node_id) {
            break;
        }
        push_next_chain_operation(iter, line);
    }
}

/// Extend a line that starts with a must chain operation.
fn extend_must_line(iter: &mut ChainOperationIter, line: &mut SmallVec<[ChainExpression; 2]>) {
    // attach immediate member and call like operations
    if matches!(iter.peek(), Some(ChainExpression::Member { .. })) {
        push_next_chain_operation(iter, line);
    }
    if matches!(
        iter.peek(),
        Some(
            ChainExpression::Call { .. }
                | ChainExpression::Index { .. }
                | ChainExpression::Instantiation { .. }
        )
    ) {
        push_next_chain_operation(iter, line);
    }
}

/// Return whether a member line should absorb a member run that ends with a call.
fn should_merge_member_run_with_call(
    context: &DestackFormatContext<'_>,
    line: &SmallVec<[ChainExpression; 2]>,
    iter: &ChainOperationIter,
) -> bool {
    if let Some(ChainExpression::Member { node_id, .. }) = line.first()
        && expression_has_line_postfix_boundary_comment(context, *node_id)
    {
        return false;
    }

    let line_has_call_like = line.iter().any(|operation| {
        matches!(
            operation,
            ChainExpression::Call { .. }
                | ChainExpression::Index { .. }
                | ChainExpression::Instantiation { .. }
        )
    });
    if line_has_call_like {
        return false;
    }

    let lookahead = iter.clone();
    let mut member_run_count = 0usize;
    let mut terminal_is_call = false;
    let mut should_merge = false;
    for next_operation in lookahead {
        match next_operation {
            ChainExpression::Member { .. } => {
                member_run_count += 1;
            }
            ChainExpression::Must { .. } => {}
            ChainExpression::Maybe { .. } => {
                break;
            }
            ChainExpression::Call { .. } => {
                terminal_is_call = true;
                should_merge = member_run_count >= 1;
                break;
            }
            ChainExpression::Index { .. } | ChainExpression::Instantiation { .. } => {
                break;
            }
        }
    }

    should_merge && terminal_is_call
}

/// Push the next chain operation into the current line when available.
fn push_next_chain_operation(
    iter: &mut std::iter::Peekable<std::vec::IntoIter<ChainExpression>>,
    line: &mut SmallVec<[ChainExpression; 2]>,
) {
    if let Some(next_operation) = iter.next() {
        line.push(next_operation);
    }
}
