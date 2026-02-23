use crate::format::chain::{
    Annotation, AnnotationPosition, ChainBreakAnalysis, ChainExpression, ChainExpressionBase,
    ChainExpressionBaseHead, DestackFormatContext, Expression, FormatError, FormatResult,
    LocalNodeId, NodeTree, NodeType, ParenthesizedUnwrapMode, PostfixPosition, SmallVec,
    analyze_chain_break, argument_is_template_literal, assignment_like_parent,
    chain_expression_from_node, chain_has_intervening_comment,
    chain_has_nonhead_nonlambda_function_call_argument, chain_has_optional_tail,
    chain_member_has_promotable_boundary_comment, chain_node_has_breaking_annotation,
    chain_node_has_non_inline_annotation, chain_nodes, expression_is_in_conditional_branch,
    is_call_like_argument, path_postfix_annotations_emit_on_tail,
    should_split_chain_root_path_segments, should_unwrap_parenthesized,
    split_chain_head_operations, transparent_inner_expression,
};
use destack_ast::{Comment, CommentStyle, Doc, DocStyle};
use smallvec::smallvec;

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

/// Return a chain base root id, unwrapping one parenthesized wrapper when safe.
fn chain_base_root_id(
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
    if !should_unwrap_parenthesized(
        context,
        root_id,
        *expression,
        ParenthesizedUnwrapMode::MemberObject,
    ) {
        return root_id;
    }

    // use the inner expression as the rendered base
    *expression
}

/// Build base head and synthetic root operations for a chain root.
fn chain_root_parts(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> FormatResult<(ChainExpressionBaseHead, Vec<ChainExpression>)> {
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

    Ok((head, operations))
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
fn should_avoid_head_promotion_for_boundary_comment(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
    body: &[ChainExpression],
) -> bool {
    let root_has_line_postfix_boundary_comment =
        expression_has_line_postfix_boundary_comment(context, root_id);
    let root_has_inline_prefix_comment_or_doc =
        chain_root_has_inline_prefix_comment_or_doc(context, root_id);
    let first_member_has_unpromotable_line_postfix_boundary_comment =
        body.first().is_some_and(|operation| {
            let ChainExpression::Member { node_id, .. } = operation else {
                return false;
            };

            expression_has_line_postfix_boundary_comment(context, *node_id)
                && !chain_member_has_promotable_boundary_comment(context, *node_id)
        });
    let starts_with_member_operation = matches!(body.first(), Some(ChainExpression::Member { .. }));

    (first_member_has_unpromotable_line_postfix_boundary_comment
        && !root_has_inline_prefix_comment_or_doc)
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
fn should_avoid_head_promotion_for_nonhead_callbacks(
    has_nonhead_nonlambda_function_call_argument: bool,
    _has_multiline_nonhead_call: bool,
    _has_chain_intervening_trivia: bool,
) -> bool {
    has_nonhead_nonlambda_function_call_argument
}

/// Promote head operations from body to base when head rules allows it.
fn promote_chain_head_operations(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    base: &mut ChainExpressionBase,
    body: &mut Vec<ChainExpression>,
    should_avoid_head_promotion_for_nonhead_callbacks: bool,
    should_avoid_head_promotion_for_boundary_comment: bool,
) {
    // skip promotion when rules say we should keep body operations split
    if should_avoid_head_promotion_for_nonhead_callbacks
        || should_avoid_head_promotion_for_boundary_comment
    {
        return;
    }

    // compute promotion inputs from current normalized base and call context
    let base_has_leading_call_like = chain_base_has_leading_call_like(context, base);
    let is_conditional_branch = expression_is_in_conditional_branch(context, node_id);
    let allow_wide_head = is_call_like_argument(context, node_id)
        || is_conditional_branch
        || assignment_like_parent(context, node_id).is_some();

    // move selected leading operations from body into base
    let head_ops_count = split_chain_head_operations(
        context,
        base_has_leading_call_like,
        body,
        allow_wide_head,
        is_conditional_branch,
    );
    if head_ops_count == 0 {
        return;
    }

    let head_operations: Vec<_> = body.drain(..head_ops_count).collect();
    base.body.extend(head_operations);
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
fn instantiation_prefix_wrap_body_ops(
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

/// Build chain base, lines, break state, and static-instantiation wrap metadata.
pub(crate) fn chain_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<(
    ChainExpressionBase,
    Vec<SmallVec<[ChainExpression; 2]>>,
    bool,
    Option<usize>,
)> {
    context.increment_counter("stats.chain.layout.builds", 1);

    // collect chain nodes from root to leaf and initialize root-derived operations
    let tree = context.tree;
    let chain = chain_nodes(tree, node_id);
    let root_id = chain[0];
    let base_root_id = chain_base_root_id(context, root_id);
    let (base_head, mut body) = chain_root_parts(context, base_root_id)?;
    let mut base = ChainExpressionBase {
        head: base_head,
        body: Vec::new(),
    };

    // append operation nodes from the original chain
    append_chain_operations(tree, &chain, &mut body)?;

    // keep a leading call-like or static-instantiation member with the base
    if let Some(first_operation) = body.first() {
        let body_has_optional_operation = body
            .iter()
            .any(|operation| matches!(operation, ChainExpression::Maybe { .. }));
        let should_promote_leading_operation =
            matches!(
                first_operation,
                ChainExpression::Call {
                    position: PostfixPosition::Direct,
                    ..
                } | ChainExpression::Instantiation { .. }
            ) || chain_operation_is_static_instantiation_member(first_operation);
        if should_promote_leading_operation {
            if !body_has_optional_operation {
                base.body.push(first_operation.clone());
                body.remove(0);
            }
        }
    }

    // analyze break signals over the original chain sequence
    let ChainBreakAnalysis {
        should_break: break_analysis_should_break,
        call_summaries: chain_call_summaries,
        has_chain_intervening_trivia,
    } = analyze_chain_break(context, &chain);
    let has_multiline_nonhead_call = chain_call_summaries
        .iter()
        .skip(1)
        .any(|summary| summary.has_multiline_argument);
    let has_nonhead_nonlambda_function_call_argument =
        chain_has_nonhead_nonlambda_function_call_argument(context, &chain);

    // resolve break gates that depend on chain member annotations and boundary comments
    let mut should_break = break_analysis_should_break;
    if chain_body_has_breaking_member_annotation(context, &body) {
        should_break = true;
    }

    let should_avoid_head_promotion_for_boundary_comment =
        should_avoid_head_promotion_for_boundary_comment(context, root_id, &body);
    let should_avoid_head_promotion_for_optional_boundary_comment = break_analysis_should_break
        && has_chain_intervening_trivia
        && chain_has_optional_tail(context, &chain)
        && chain_has_intervening_comment(context, &chain);
    if should_avoid_head_promotion_for_boundary_comment {
        should_break = true;
    }
    if should_avoid_head_promotion_for_optional_boundary_comment {
        should_break = true;
    }

    // promote eligible head operations from body into base
    let should_avoid_head_promotion_for_nonhead_callbacks =
        should_avoid_head_promotion_for_nonhead_callbacks(
            has_nonhead_nonlambda_function_call_argument,
            has_multiline_nonhead_call,
            has_chain_intervening_trivia,
        );

    promote_chain_head_operations(
        context,
        node_id,
        &mut base,
        &mut body,
        should_avoid_head_promotion_for_nonhead_callbacks,
        should_avoid_head_promotion_for_boundary_comment
            || should_avoid_head_promotion_for_optional_boundary_comment,
    );

    // group chain operations and then apply argument-chain compaction rules
    let mut lines = group_chain_expression_lines(context, body);
    apply_chain_line_promotions(context, node_id, &mut base, &mut lines);
    promote_leading_grouped_instantiation_prefix(context, &mut base, &mut lines);
    let instantiation_prefix_wrap_body_ops =
        instantiation_prefix_wrap_body_ops(context, &base, &lines);

    Ok((
        base,
        lines,
        should_break,
        instantiation_prefix_wrap_body_ops,
    ))
}

/// Return whether an expression has a line postfix boundary comment annotation.
pub(crate) fn expression_has_line_postfix_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(node_id, |annotations| {
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
pub(crate) fn group_chain_expression_lines(
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

/// Return whether lookahead starts with a simple `?.member` tail pair.
fn iter_starts_with_simple_optional_member_tail_pair(
    context: &DestackFormatContext<'_>,
    iter: &ChainOperationIter,
) -> bool {
    let mut lookahead = iter.clone();

    let Some(ChainExpression::Maybe { node_id, .. }) = lookahead.next() else {
        return false;
    };
    if chain_node_has_non_inline_annotation(context, node_id) {
        return false;
    }

    let Some(ChainExpression::Member { node_id, .. }) = lookahead.next() else {
        return false;
    };
    if chain_node_has_non_inline_annotation(context, node_id) {
        return false;
    }

    !matches!(
        lookahead.peek(),
        Some(
            ChainExpression::Call { .. }
                | ChainExpression::Index { .. }
                | ChainExpression::Instantiation { .. }
                | ChainExpression::Must { .. }
        )
    )
}

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

        // keep short optional member tails with the preceding optional call
        let mut merged_optional_member_tail_pair_count = 0usize;
        while merged_optional_member_tail_pair_count < 2
            && iter_starts_with_simple_optional_member_tail_pair(context, iter)
        {
            push_next_chain_operation(iter, line);
            push_next_chain_operation(iter, line);
            merged_optional_member_tail_pair_count += 1;
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
