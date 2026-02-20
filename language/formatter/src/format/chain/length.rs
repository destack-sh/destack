use super::{
    Annotation, AnnotationPosition, Argument, ChainExpression, ChainExpressionBase,
    ChainExpressionBaseHead, DestackFormatContext, Expression, LocalNodeId, PostfixPosition, Span,
    TokenType, argument_has_non_blank_annotation, arguments_rendered_len,
    call_arguments_force_expand_for_chain, call_has_non_blank_infix_annotation,
    chain_node_has_non_inline_annotation, expression_inline_width_hint,
    is_simple_chain_static_arguments,
};

/// Return the concrete span that corresponds to one annotation node.
fn annotation_content_span(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Span {
    context.annotation_span(annotation_id)
}
/// Check whether a member access uses a private hash (`.#name`).
pub(crate) fn member_is_private_hash(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(context.tree.get(node_id), Expression::PrivateMember { .. }) {
        return true;
    }

    let Some(property_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    let token_idx = context
        .tokens
        .iter()
        .position(|token| token.span.start == property_span.start);
    let Some(token_idx) = token_idx else {
        return false;
    };

    let prev_token = token_idx
        .checked_sub(1)
        .and_then(|index| context.tokens.get(index));
    let Some(prev_token) = prev_token else {
        return false;
    };

    prev_token.token.ty == TokenType::Hash
}

/// Decide whether postfix annotations on a path belong after the last segment.
pub(crate) fn path_postfix_annotations_emit_on_tail(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> bool {
    let Some(last_segment_start) = path_last_segment_start(context, node_id, segments_len) else {
        return false;
    };

    context
        .visit_annotations(node_id, |annotations| {
            let mut has_postfix = false;
            for annotation_id in annotations {
                let annotation = context.annotation(*annotation_id);
                let position = annotation.position();
                let is_postfix = matches!(
                    position,
                    AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                );
                if !is_postfix {
                    continue;
                }

                has_postfix = true;
                let span = annotation_content_span(context, *annotation_id);
                if span.start < last_segment_start {
                    return false;
                }
            }

            if !has_postfix {
                return true;
            }

            true
        })
        .unwrap_or(true)
}

/// Find the start byte of the last path segment token.
pub(crate) fn path_last_segment_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> Option<u32> {
    if segments_len == 0 {
        return None;
    }

    let span = context.span(node_id);
    let mut count = 0usize;
    for token in context.tokens.iter() {
        if token.span.start < span.start {
            continue;
        }
        if token.span.start >= span.end {
            break;
        }
        if token.token.ty == TokenType::Identifier {
            count += 1;
            if count == segments_len {
                return Some(token.span.start);
            }
        }
    }

    None
}

/// Estimate the rendered length of static arguments.
pub(crate) fn static_arguments_len(
    context: &DestackFormatContext<'_>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
) -> usize {
    match static_arguments {
        None => 0,
        Some(arguments) => static_argument_list_len(context, arguments),
    }
}

/// Estimate the rendered length of a static argument list.
pub(crate) fn static_argument_list_len(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> usize {
    // compute argument length inside delimiters
    let arguments_len = arguments_rendered_len(context, static_arguments);

    // account for `<` and `>`
    arguments_len.saturating_add(2)
}

/// Estimate the rendered length of a single chain operation.
pub(crate) fn chain_operation_len(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> usize {
    match operation {
        ChainExpression::Member {
            segment,
            static_arguments,
            ..
        } => {
            // measure the segment name
            let segment_len = context.strings.get(*segment).chars().count();

            // include any static arguments
            let static_len = static_arguments_len(context, static_arguments);

            // account for `.` plus the content
            1usize
                .saturating_add(segment_len)
                .saturating_add(static_len)
        }
        ChainExpression::Instantiation {
            static_arguments, ..
        } => static_argument_list_len(context, static_arguments),
        ChainExpression::Call {
            position,
            static_arguments,
            dynamic_arguments,
            ..
        } => {
            // account for a leading `.` on indirect calls
            let dot_len = usize::from(*position == PostfixPosition::Indirect);

            // include static arguments
            let static_len = static_arguments_len(context, static_arguments);

            // measure the arguments within parentheses
            let arguments_len = arguments_rendered_len(context, dynamic_arguments);

            // include `(` and `)`
            dot_len
                .saturating_add(static_len)
                .saturating_add(2)
                .saturating_add(arguments_len)
        }
        ChainExpression::Index {
            position, index, ..
        } => {
            // account for a leading `.` on indirect indexes
            let dot_len = usize::from(*position == PostfixPosition::Indirect);

            // measure the index expression if it exists
            let index_len = match index {
                Some(index_id) => expression_inline_width_hint(context, *index_id),
                None => 0,
            };

            // include `[` and `]`
            dot_len.saturating_add(2).saturating_add(index_len)
        }
        ChainExpression::Maybe { position, .. } => match position {
            PostfixPosition::Direct => 1,
            PostfixPosition::Indirect => 2,
        },
        ChainExpression::Must { position, .. } => match position {
            PostfixPosition::Direct => 1,
            PostfixPosition::Indirect => 2,
        },
    }
}

/// Return whether a chain call can stay in the head even when its arguments expand.
pub(crate) fn chain_call_can_expand_in_head(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if call_has_non_blank_infix_annotation(context, call_node_id) {
        return false;
    }

    if chain_node_has_non_inline_annotation(context, call_node_id) {
        return false;
    }

    if !is_simple_chain_static_arguments(context, static_arguments) {
        return false;
    }

    if dynamic_arguments
        .iter()
        .any(|argument_id| argument_has_non_blank_annotation(context, *argument_id))
    {
        return false;
    }

    call_arguments_force_expand_for_chain(context, call_node_id, dynamic_arguments)
}

/// Estimate the rendered length of a chain operation when promoted into the head.
pub(crate) fn chain_head_operation_len(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> usize {
    match operation {
        ChainExpression::Call {
            node_id,
            position,
            static_arguments,
            dynamic_arguments,
        } => {
            if chain_call_can_expand_in_head(context, *node_id, static_arguments, dynamic_arguments)
            {
                let dot_len = usize::from(*position == PostfixPosition::Indirect);
                let static_len = static_arguments_len(context, static_arguments);
                dot_len.saturating_add(static_len).saturating_add(1)
            } else {
                chain_operation_len(context, operation)
            }
        }
        _ => chain_operation_len(context, operation),
    }
}

/// Estimate the rendered length of the chain base.
pub(crate) fn chain_base_len(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
) -> usize {
    // measure the base head
    let mut head_len = match &base.head {
        ChainExpressionBaseHead::Path {
            node_id: _,
            segment,
            static_arguments,
            emit_postfix_annotations: _,
        } => {
            let segment_len = context.strings.get(*segment).chars().count();
            let static_len = static_arguments_len(context, static_arguments);
            segment_len.saturating_add(static_len)
        }
        ChainExpressionBaseHead::Expression(node_id) => {
            expression_inline_width_hint(context, *node_id)
        }
    };

    // add any base operations
    for operation in &base.body {
        let operation_len = chain_head_operation_len(context, operation);
        head_len = head_len.saturating_add(operation_len);
    }

    head_len
}
