use super::super::{
    DestackFormatContext, Expression, LocalNodeId, NodeType, Span,
    chain_has_parent_intervening_break_or_comment, expression_trivia_anchor_end,
    has_comment_between_expressions, member_has_intervening_break_or_comment,
    member_has_intervening_comment, span_has_comment,
};

/// Return whether a chain contains trivia between adjacent chain operations.
pub(super) fn chain_has_intervening_break_or_comment(
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
pub(super) fn chain_has_intervening_comment(
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
