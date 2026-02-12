use super::*;

use super::line_group::{
    expression_has_line_postfix_boundary_comment, group_chain_expression_lines,
};

/// Store the root-derived base and synthetic operations for chain formatting.
struct ChainRootParts {
    head: ChainExpressionBaseHead,
    operations: Vec<ChainExpression>,
    deferred_boundary_comments: Vec<String>,
}

/// Store normalized chain inputs before layout scoring.
struct NormalizedChainLayout {
    chain: Vec<LocalNodeId<Expression>>,
    root_id: LocalNodeId<Expression>,
    base: ChainExpressionBase,
    body: Vec<ChainExpression>,
    deferred_path_boundary_comments: Vec<String>,
}

/// Store planned chain layout used by render-only formatting.
pub(super) struct ChainLayoutPlan {
    pub(super) base: ChainExpressionBase,
    pub(super) lines: Vec<SmallVec<[ChainExpression; 2]>>,
    pub(super) deferred_path_boundary_comments: Vec<String>,
    pub(super) should_break: bool,
    pub(super) has_calls: bool,
    pub(super) in_template_literal_interpolation: bool,
}

/// Build base head and synthetic root operations for a chain root.
fn collect_chain_root_parts(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> FormatResult<ChainRootParts> {
    let tree = context.tree;
    let mut head = ChainExpressionBaseHead::Expression(root_id);
    let mut operations = Vec::new();
    let mut deferred_boundary_comments = Vec::new();

    // split root path segments into explicit member chain operations
    if let Expression::Path {
        path,
        static_arguments,
    } = tree.get(root_id)
        && should_split_chain_root_path_segments(context, root_id)
    {
        let segments = &path.segments;
        let static_arguments = static_arguments.clone();
        let Some(first_segment) = segments.first().copied() else {
            return Err(FormatError::SyntaxError {
                message: "path chain root must contain at least one segment",
            });
        };

        let tail_segments = &segments[1..];
        let tail_len = tail_segments.len();
        if tail_len > 0 {
            deferred_boundary_comments =
                path_deferred_boundary_line_comments(context, root_id, segments.len());
        }

        let emit_postfix_on_tail =
            tail_len > 0 && path_postfix_annotations_emit_on_tail(context, root_id, segments.len());

        // path base keeps static arguments only when there is no synthetic tail
        let base_static_arguments = if tail_len == 0 {
            static_arguments.clone()
        } else {
            None
        };
        head = ChainExpressionBaseHead::Path {
            node_id: root_id,
            segment: first_segment,
            static_arguments: base_static_arguments,
            emit_postfix_annotations: (tail_len == 0 || !emit_postfix_on_tail)
                && deferred_boundary_comments.is_empty(),
        };

        // append synthetic member operations for remaining path segments
        for (index, segment) in tail_segments.iter().copied().enumerate() {
            let is_last = index + 1 == tail_len;
            let static_args = if is_last {
                static_arguments.clone()
            } else {
                None
            };
            operations.push(ChainExpression::Member {
                node_id: root_id,
                segment,
                static_arguments: static_args,
                emit_prefix_annotations: false,
                emit_postfix_annotations: emit_postfix_on_tail && is_last,
            });
        }
    }

    Ok(ChainRootParts {
        head,
        operations,
        deferred_boundary_comments,
    })
}

/// Append operation nodes from chain body expressions.
fn append_chain_operations(
    tree: &NodeTree,
    chain: &[LocalNodeId<Expression>],
    body: &mut Vec<ChainExpression>,
) -> FormatResult<()> {
    for expression_id in chain.iter().skip(1).copied() {
        body.push(chain_expression_from_node(tree, expression_id)?);
    }

    Ok(())
}

/// Return whether normalized chain body has a member with non-inline annotations.
fn chain_body_has_breaking_member_annotation(
    context: &DestackFormatContext<'_>,
    body: &[ChainExpression],
) -> bool {
    body.iter().any(|operation| {
        let ChainExpression::Member { node_id, .. } = operation else {
            return false;
        };

        chain_node_has_breaking_annotation(context, *node_id)
    })
}

/// Return whether boundary comment rules should block head promotion.
fn chain_should_avoid_head_promotion_for_boundary_comment(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
    body: &[ChainExpression],
) -> bool {
    let root_has_line_postfix_boundary_comment =
        expression_has_line_postfix_boundary_comment(context, root_id);
    let first_member_has_line_postfix_boundary_comment = body.first().is_some_and(|operation| {
        let ChainExpression::Member { node_id, .. } = operation else {
            return false;
        };

        expression_has_line_postfix_boundary_comment(context, *node_id)
    });
    let starts_with_member_operation = matches!(body.first(), Some(ChainExpression::Member { .. }));

    first_member_has_line_postfix_boundary_comment
        || (root_has_line_postfix_boundary_comment && starts_with_member_operation)
}

/// Return whether a normalized chain base begins with call-like operations.
fn chain_base_has_leading_call_like(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
) -> bool {
    match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => matches!(
            context.tree.get(*expression_id),
            Expression::Call { .. } | Expression::Instantiation { .. }
        ),
        ChainExpressionBaseHead::Path { .. } => base.body.first().is_some_and(|operation| {
            matches!(
                operation,
                ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
            )
        }),
    }
}

/// Return assignment-like remaining width when head promotion may use it.
fn chain_head_promotion_remaining_width(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<usize> {
    if is_call_like_argument(context, node_id)
        || expression_is_in_conditional_branch(context, node_id)
    {
        return None;
    }

    assignment_like_remaining_width(context, node_id)
}

/// Return whether non-head callback signals should block head promotion.
fn chain_should_avoid_head_promotion_for_nonhead_callbacks(
    context: &DestackFormatContext<'_>,
    normalized: &NormalizedChainLayout,
    has_nonhead_nonlambda_function_call_argument: bool,
    has_multiline_nonhead_call: bool,
    has_chain_intervening_trivia: bool,
    has_path_tail_deferred_empty_call_boundary_comment: bool,
) -> bool {
    let root_has_annotation = context.has_annotation(normalized.root_id);

    has_nonhead_nonlambda_function_call_argument
        || (has_multiline_nonhead_call && has_chain_intervening_trivia)
        || has_path_tail_deferred_empty_call_boundary_comment
        || root_has_annotation
}

/// Promote head operations from body to base when head policy allows it.
fn promote_chain_head_operations(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    normalized: &mut NormalizedChainLayout,
    should_avoid_head_promotion_for_nonhead_callbacks: bool,
    should_avoid_head_promotion_for_boundary_comment: bool,
) {
    // skip promotion when policy says we should keep body operations split
    if should_avoid_head_promotion_for_nonhead_callbacks
        || should_avoid_head_promotion_for_boundary_comment
    {
        return;
    }

    // compute promotion inputs from current normalized base and call context
    let base_len = chain_base_len(context, &normalized.base);
    let base_has_leading_call_like = chain_base_has_leading_call_like(context, &normalized.base);
    let remaining_width = chain_head_promotion_remaining_width(context, node_id);
    let allow_wide_head = is_call_like_argument(context, node_id);

    // move selected leading operations from body into base
    let head_ops_count = split_chain_head_operations(
        context,
        base_len,
        base_has_leading_call_like,
        &normalized.body,
        remaining_width,
        allow_wide_head,
    );
    if head_ops_count == 0 {
        return;
    }

    let head_operations: Vec<_> = normalized.body.drain(..head_ops_count).collect();
    normalized.base.body.extend(head_operations);
}

/// Promote a direct-call first line into base for curried call stability.
fn promote_curried_call_tail_line(
    context: &DestackFormatContext<'_>,
    base: &mut ChainExpressionBase,
    lines: &mut Vec<SmallVec<[ChainExpression; 2]>>,
) {
    // only promote when base already ends in a direct call
    let base_ends_with_direct_call = base.body.last().is_some_and(|operation| {
        matches!(
            operation,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            }
        )
    });
    if !base_ends_with_direct_call {
        return;
    }

    // only promote a single direct call first line without breaking annotations
    let Some(first_line) = lines.first() else {
        return;
    };
    if first_line.len() != 1 {
        return;
    }

    let ChainExpression::Call {
        node_id,
        position: PostfixPosition::Direct,
        ..
    } = first_line[0]
    else {
        return;
    };
    if chain_node_has_non_inline_annotation(context, node_id) {
        return;
    }

    let first_line = lines.remove(0);
    base.body.push(first_line[0].clone());
}

/// Promote a single first-line call after a trailing base member in argument chains.
fn promote_argument_member_call_head_line(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    base: &mut ChainExpressionBase,
    lines: &mut Vec<SmallVec<[ChainExpression; 2]>>,
) {
    // only apply this compaction for argument-position chains
    if !is_call_like_argument(context, node_id) {
        return;
    }

    // only apply when base currently ends with a member operation
    let base_ends_with_member = base
        .body
        .last()
        .is_some_and(|operation| matches!(operation, ChainExpression::Member { .. }));
    if !base_ends_with_member {
        return;
    }

    // only promote a single clean call operation
    let Some(first_line) = lines.first() else {
        return;
    };
    if first_line.len() != 1 {
        return;
    }

    let ChainExpression::Call {
        node_id: call_node_id,
        ..
    } = first_line[0]
    else {
        return;
    };
    if chain_node_has_non_inline_annotation(context, call_node_id) {
        return;
    }

    let first_line = lines.remove(0);
    base.body.push(first_line[0].clone());
}

/// Promote a first-line member plus call pair after trailing base member in argument chains.
fn promote_argument_member_call_pair_line(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    base: &mut ChainExpressionBase,
    lines: &mut Vec<SmallVec<[ChainExpression; 2]>>,
) {
    // only apply this compaction for argument-position chains
    if !is_call_like_argument(context, node_id) {
        return;
    }

    // only apply when base currently ends with a member operation
    let base_ends_with_member = base
        .body
        .last()
        .is_some_and(|operation| matches!(operation, ChainExpression::Member { .. }));
    if !base_ends_with_member {
        return;
    }

    // only promote a clean member-plus-call pair
    let Some(first_line) = lines.first() else {
        return;
    };
    if first_line.len() != 2 {
        return;
    }

    let (
        ChainExpression::Member {
            node_id: member_node_id,
            ..
        },
        ChainExpression::Call {
            node_id: call_node_id,
            ..
        },
    ) = (&first_line[0], &first_line[1])
    else {
        return;
    };
    if chain_node_has_non_inline_annotation(context, *member_node_id)
        || chain_node_has_non_inline_annotation(context, *call_node_id)
    {
        return;
    }

    let first_line = lines.remove(0);
    base.body.extend(first_line);
}

/// Merge short leading argument chain lines to avoid fragmented member hops.
fn merge_argument_short_member_hop_lines(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    lines: &mut Vec<SmallVec<[ChainExpression; 2]>>,
) {
    // only apply this compaction for argument-position chains
    if !is_call_like_argument(context, node_id) {
        return;
    }

    // only merge when there are at least two lines with compatible starts
    if lines.len() < 2 {
        return;
    }

    if !matches!(lines[0].as_slice(), [ChainExpression::Member { .. }]) {
        return;
    }

    if !matches!(
        lines[1].first(),
        Some(ChainExpression::Member { .. } | ChainExpression::Call { .. })
    ) {
        return;
    }

    let mut first_line = lines.remove(0);
    let second_line = lines.remove(0);
    first_line.extend(second_line);
    lines.insert(0, first_line);
}

/// Apply line-level argument chain promotions after grouping.
fn apply_argument_chain_line_promotions(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    base: &mut ChainExpressionBase,
    lines: &mut Vec<SmallVec<[ChainExpression; 2]>>,
) {
    // preserve curried direct-call tails after head promotion
    promote_curried_call_tail_line(context, base, lines);

    // keep short member-plus-call heads compact in argument positions
    promote_argument_member_call_head_line(context, node_id, base, lines);
    promote_argument_member_call_pair_line(context, node_id, base, lines);

    // merge small leading member hops to avoid fragmented argument chains
    merge_argument_short_member_hop_lines(context, node_id, lines);
}

/// Normalize a chain root into base and operation inputs for planning.
fn normalize_chain_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<NormalizedChainLayout> {
    let tree = context.tree;

    // collect chain nodes from root to leaf
    let chain = collect_chain_nodes(tree, node_id);
    let root_id = chain[0];

    // initialize base head and synthetic root path operations
    let root_parts = collect_chain_root_parts(context, root_id)?;
    let mut body = root_parts.operations;
    let mut base = ChainExpressionBase {
        head: root_parts.head,
        body: Vec::new(),
    };
    let deferred_path_boundary_comments = root_parts.deferred_boundary_comments;

    // append operation nodes from the original chain
    append_chain_operations(tree, &chain, &mut body)?;

    // keep a leading call with the base so alignment stays stable
    if let Some(first_op) = body.first()
        && matches!(
            first_op,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            } | ChainExpression::Instantiation { .. }
        )
    {
        base.body.push(first_op.clone());
        body.remove(0);
    }

    Ok(NormalizedChainLayout {
        chain,
        root_id,
        base,
        body,
        deferred_path_boundary_comments,
    })
}

/// Build a scored chain layout plan that rendering can consume directly.
pub(super) fn plan_chain_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<ChainLayoutPlan> {
    context.increment_counter("profile.chain.layout.builds", 1);

    // gather normalized inputs and break analysis
    let mut normalized = normalize_chain_layout(context, node_id)?;
    let ChainBreakAnalysis {
        should_break: break_analysis_should_break,
        call_summaries: chain_call_summaries,
        has_chain_intervening_trivia,
        has_path_tail_deferred_empty_call_boundary_comment,
    } = analyze_chain_break(context, &normalized.chain);
    let has_multiline_nonhead_call = chain_call_summaries
        .iter()
        .skip(1)
        .any(|summary| summary.has_multiline_argument);
    let has_nonhead_nonlambda_function_call_argument =
        chain_has_nonhead_nonlambda_function_call_argument(context, &normalized.chain);

    // resolve break gates that depend on chain member annotations and boundary comments
    let mut should_break = break_analysis_should_break;
    if chain_body_has_breaking_member_annotation(context, &normalized.body) {
        should_break = true;
    }

    let should_avoid_head_promotion_for_boundary_comment =
        chain_should_avoid_head_promotion_for_boundary_comment(
            context,
            normalized.root_id,
            &normalized.body,
        );
    if should_avoid_head_promotion_for_boundary_comment {
        should_break = true;
    }

    // promote eligible head operations from body into base
    let should_avoid_head_promotion_for_nonhead_callbacks =
        chain_should_avoid_head_promotion_for_nonhead_callbacks(
            context,
            &normalized,
            has_nonhead_nonlambda_function_call_argument,
            has_multiline_nonhead_call,
            has_chain_intervening_trivia,
            has_path_tail_deferred_empty_call_boundary_comment,
        );
    promote_chain_head_operations(
        context,
        node_id,
        &mut normalized,
        should_avoid_head_promotion_for_nonhead_callbacks,
        should_avoid_head_promotion_for_boundary_comment,
    );

    // group chain operations and then apply argument-chain compaction rules
    let mut lines = group_chain_expression_lines(context, normalized.body);
    apply_argument_chain_line_promotions(context, node_id, &mut normalized.base, &mut lines);

    Ok(ChainLayoutPlan {
        base: normalized.base,
        lines,
        deferred_path_boundary_comments: normalized.deferred_path_boundary_comments,
        should_break,
        has_calls: !chain_call_summaries.is_empty(),
        in_template_literal_interpolation: expression_is_in_template_literal_interpolation(
            context, node_id,
        ),
    })
}
