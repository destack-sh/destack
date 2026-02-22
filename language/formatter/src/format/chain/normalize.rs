use crate::format::chain::{
    Annotation, AnnotationPosition, ChainBreakAnalysis, ChainExpression, ChainExpressionBase,
    ChainExpressionBaseHead, DestackFormatContext, Expression, FormatError, FormatResult,
    LocalNodeId, NodeTree, NodeType, ParenthesizedUnwrapPolicy, PostfixPosition, SmallVec,
    analyze_chain_break, assignment_like_parent, chain_expression_from_node,
    chain_has_nonhead_nonlambda_function_call_argument, chain_node_has_breaking_annotation,
    chain_node_has_non_inline_annotation, collect_chain_nodes, expression_is_in_conditional_branch,
    is_call_like_argument, parenthesized_should_unwrap, path_postfix_annotations_emit_on_tail,
    should_split_chain_root_path_segments, split_chain_head_operations,
    transparent_inner_expression,
};
use destack_ast::{Comment, CommentStyle, Doc, DocStyle};

use crate::format::chain::line_group::{
    expression_has_line_postfix_boundary_comment, group_chain_expression_lines,
};

/// Return whether one chain root has inline doc/comment prefix annotations.
fn chain_root_has_inline_prefix_comment_or_doc(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(root_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                if !matches!(
                    annotation.position(),
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return false;
                }

                match annotation {
                    Annotation::Comment { node, .. } => {
                        context.tree.get::<Comment>(node).style == CommentStyle::Star
                    }
                    Annotation::Doc { node, .. } => {
                        context.tree.get::<Doc>(node).style == DocStyle::Star
                    }
                    Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
                }
            })
        })
        .unwrap_or(false)
}

/// Store the root-derived base and synthetic operations for chain formatting.
struct ChainRootParts {
    head: ChainExpressionBaseHead,
    operations: Vec<ChainExpression>,
}

/// Store normalized chain inputs before layout scoring.
struct NormalizedChainLayout {
    chain: Vec<LocalNodeId<Expression>>,
    root_id: LocalNodeId<Expression>,
    base: ChainExpressionBase,
    body: Vec<ChainExpression>,
}

/// Return a chain base root id, unwrapping one parenthesized wrapper when safe.
fn resolve_chain_base_root_id(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    // only parenthesized roots are candidates for unwrap
    let Expression::Parenthesized { expression } = context.tree.get(root_id) else {
        return root_id;
    };

    // keep object-literal wrappers in statement position:
    // `({}).x` cannot become `{}.x`
    let root_is_statement_expression =
        context
            .parent(root_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }

                matches!(
                    context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Statement(expression_id) if expression_id.id == root_id.id
                )
            });
    if root_is_statement_expression
        && matches!(
            context.tree.get(*expression),
            Expression::ObjectExpression { .. }
        )
    {
        return root_id;
    }

    // keep wrappers that are required in postfix contexts
    if !parenthesized_should_unwrap(
        context,
        root_id,
        *expression,
        ParenthesizedUnwrapPolicy::MemberObject,
    ) {
        return root_id;
    }

    // use the inner expression as the rendered base
    *expression
}

/// Store planned chain layout used by render-only formatting.
pub(crate) struct ChainLayoutPlan {
    pub(crate) base: ChainExpressionBase,
    pub(crate) lines: Vec<SmallVec<[ChainExpression; 2]>>,
    pub(crate) should_break: bool,
    pub(crate) instantiation_prefix_wrap_body_ops: Option<usize>,
}

/// Build base head and synthetic root operations for a chain root.
fn collect_chain_root_parts(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> FormatResult<ChainRootParts> {
    let tree = context.tree;
    let mut head = ChainExpressionBaseHead::Expression(root_id);
    let mut operations = Vec::new();

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
            emit_postfix_annotations: tail_len == 0 || !emit_postfix_on_tail,
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

    Ok(ChainRootParts { head, operations })
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
    let root_has_inline_prefix_comment_or_doc =
        chain_root_has_inline_prefix_comment_or_doc(context, root_id);
    let first_member_has_line_postfix_boundary_comment = body.first().is_some_and(|operation| {
        let ChainExpression::Member { node_id, .. } = operation else {
            return false;
        };

        expression_has_line_postfix_boundary_comment(context, *node_id)
    });
    let starts_with_member_operation = matches!(body.first(), Some(ChainExpression::Member { .. }));

    (first_member_has_line_postfix_boundary_comment && !root_has_inline_prefix_comment_or_doc)
        || (root_has_line_postfix_boundary_comment && starts_with_member_operation)
}

/// Return whether a normalized chain base begins with call-like operations.
fn chain_base_has_leading_call_like(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
) -> bool {
    match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => {
            let expression_id = transparent_inner_expression(context, *expression_id);
            let expression = context.tree.get(expression_id);
            if matches!(
                expression,
                Expression::Call { .. } | Expression::Instantiation { .. }
            ) {
                return true;
            }

            matches!(
                expression,
                Expression::Parenthesized { expression }
                    if matches!(
                        context.tree.get(*expression),
                        Expression::Call { .. } | Expression::Instantiation { .. }
                    )
            ) || base.body.first().is_some_and(|operation| {
                matches!(
                    operation,
                    ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
                )
            })
        }
        ChainExpressionBaseHead::Path { .. } => base.body.first().is_some_and(|operation| {
            matches!(
                operation,
                ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
            )
        }),
    }
}

/// Return whether non-head callback signals should block head promotion.
fn chain_should_avoid_head_promotion_for_nonhead_callbacks(
    has_nonhead_nonlambda_function_call_argument: bool,
    has_multiline_nonhead_call: bool,
    has_chain_intervening_trivia: bool,
) -> bool {
    has_nonhead_nonlambda_function_call_argument
        || (has_multiline_nonhead_call && has_chain_intervening_trivia)
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
    let base_has_leading_call_like = chain_base_has_leading_call_like(context, &normalized.base);
    let is_conditional_branch = expression_is_in_conditional_branch(context, node_id);
    let allow_wide_head = is_call_like_argument(context, node_id)
        || is_conditional_branch
        || assignment_like_parent(context, node_id).is_some();

    // move selected leading operations from body into base
    let head_ops_count = split_chain_head_operations(
        context,
        base_has_leading_call_like,
        &normalized.body,
        allow_wide_head,
        is_conditional_branch,
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

/// Promote a leading direct index line into the chain base.
fn promote_leading_direct_index_line(
    context: &DestackFormatContext<'_>,
    base: &mut ChainExpressionBase,
    lines: &mut Vec<SmallVec<[ChainExpression; 2]>>,
) {
    // only promote when the first grouped line starts with a direct index op
    let Some(first_line) = lines.first() else {
        return;
    };
    let Some(ChainExpression::Index {
        node_id,
        position: PostfixPosition::Direct,
        ..
    }) = first_line.first()
    else {
        return;
    };

    // keep annotated index operations isolated to avoid comment churn
    if chain_node_has_non_inline_annotation(context, *node_id) {
        return;
    }

    // move the first line into the base to avoid ASI-sensitive leading `[` lines
    let first_line = lines.remove(0);
    base.body.extend(first_line);
}

/// Apply chain line promotions after grouping.
fn apply_chain_line_promotions(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    base: &mut ChainExpressionBase,
    lines: &mut Vec<SmallVec<[ChainExpression; 2]>>,
) {
    // prevent ASI-sensitive leading `[` lines from splitting the chain
    promote_leading_direct_index_line(context, base, lines);

    // preserve curried direct-call tails after head promotion
    promote_curried_call_tail_line(context, base, lines);

    // keep short member-plus-call heads compact in argument positions
    promote_argument_member_call_head_line(context, node_id, base, lines);
    promote_argument_member_call_pair_line(context, node_id, base, lines);

    // merge small leading member hops to avoid fragmented argument chains
    merge_argument_short_member_hop_lines(context, node_id, lines);
}

/// Return the expression node id carried by one chain operation.
fn chain_operation_node_id(operation: &ChainExpression) -> LocalNodeId<Expression> {
    match operation {
        ChainExpression::Member { node_id, .. }
        | ChainExpression::Instantiation { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Maybe { node_id, .. }
        | ChainExpression::Must { node_id, .. } => *node_id,
    }
}

/// Return whether one chain operation carries static instantiation arguments.
fn chain_operation_has_static_instantiation_arguments(operation: &ChainExpression) -> bool {
    match operation {
        ChainExpression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        ChainExpression::Member {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        _ => false,
    }
}

/// Return whether one operation is a member carrying static instantiation arguments.
fn chain_operation_is_static_instantiation_member(operation: &ChainExpression) -> bool {
    matches!(
        operation,
        ChainExpression::Member {
            static_arguments: Some(arguments),
            ..
        } if !arguments.is_empty()
    )
}

/// Promote the first grouped instantiation prefix segment into the chain base.
fn promote_leading_grouped_instantiation_prefix(
    context: &DestackFormatContext<'_>,
    base: &mut ChainExpressionBase,
    lines: &mut Vec<SmallVec<[ChainExpression; 2]>>,
) {
    let Some(first_line) = lines.first_mut() else {
        return;
    };

    let Some(prefix_end_index) = first_line
        .iter()
        .position(chain_operation_has_static_instantiation_arguments)
    else {
        return;
    };

    let prefix_is_supported = first_line.iter().take(prefix_end_index).all(|operation| {
        matches!(
            operation,
            ChainExpression::Member { .. } | ChainExpression::Index { .. }
        )
    });
    if !prefix_is_supported {
        return;
    }

    let prefix_has_non_inline_annotation =
        first_line
            .iter()
            .take(prefix_end_index + 1)
            .any(|operation| {
                chain_node_has_non_inline_annotation(context, chain_operation_node_id(operation))
            });
    if prefix_has_non_inline_annotation {
        return;
    }

    let promoted_prefix: Vec<_> = first_line.drain(..=prefix_end_index).collect();
    base.body.extend(promoted_prefix);
    if first_line.is_empty() {
        lines.remove(0);
    }
}

/// Return whether one chain base head expression carries trailing static instantiation arguments.
fn chain_expression_has_trailing_static_instantiation_arguments(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        Expression::Path {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Parenthesized { expression } => {
            chain_expression_has_trailing_static_instantiation_arguments(tree, *expression)
        }
        _ => false,
    }
}

/// Return one base-body wrap prefix count for static instantiation member tail wrapping.
fn chain_instantiation_prefix_wrap_body_ops(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
) -> Option<usize> {
    let head_has_static_instantiation_prefix = match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => {
            chain_expression_has_trailing_static_instantiation_arguments(
                context.tree,
                *expression_id,
            )
        }
        ChainExpressionBaseHead::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
    };
    let static_instantiation_body_index = base
        .body
        .iter()
        .position(chain_operation_has_static_instantiation_arguments);
    let Some(prefix_body_ops) = (if head_has_static_instantiation_prefix {
        Some(0usize)
    } else {
        static_instantiation_body_index.map(|index| index + 1)
    }) else {
        return None;
    };

    let has_member_tail_in_base = base
        .body
        .iter()
        .skip(prefix_body_ops)
        .any(|operation| matches!(operation, ChainExpression::Member { .. }));
    let has_member_tail_in_lines = lines
        .first()
        .and_then(|line| line.first())
        .is_some_and(|operation| matches!(operation, ChainExpression::Member { .. }));
    if !(has_member_tail_in_base || has_member_tail_in_lines) {
        return None;
    }

    Some(prefix_body_ops)
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
    let base_root_id = resolve_chain_base_root_id(context, root_id);

    // initialize base head and synthetic root path operations
    let root_parts = collect_chain_root_parts(context, base_root_id)?;
    let mut body = root_parts.operations;
    let mut base = ChainExpressionBase {
        head: root_parts.head,
        body: Vec::new(),
    };

    // append operation nodes from the original chain
    append_chain_operations(tree, &chain, &mut body)?;

    // keep a leading call-like or static-instantiation member with the base
    if let Some(first_op) = body.first() {
        let should_promote_leading_operation =
            matches!(
                first_op,
                ChainExpression::Call {
                    position: PostfixPosition::Direct,
                    ..
                } | ChainExpression::Instantiation { .. }
            ) || chain_operation_is_static_instantiation_member(first_op);
        if should_promote_leading_operation {
            base.body.push(first_op.clone());
            body.remove(0);
        }
    }

    Ok(NormalizedChainLayout {
        chain,
        root_id,
        base,
        body,
    })
}

/// Build a scored chain layout plan that rendering can consume directly.
pub(crate) fn plan_chain_layout(
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
            has_nonhead_nonlambda_function_call_argument,
            has_multiline_nonhead_call,
            has_chain_intervening_trivia,
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
    apply_chain_line_promotions(context, node_id, &mut normalized.base, &mut lines);
    promote_leading_grouped_instantiation_prefix(context, &mut normalized.base, &mut lines);
    let instantiation_prefix_wrap_body_ops =
        chain_instantiation_prefix_wrap_body_ops(context, &normalized.base, &lines);

    Ok(ChainLayoutPlan {
        base: normalized.base,
        lines,
        should_break,
        instantiation_prefix_wrap_body_ops,
    })
}
