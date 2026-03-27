use crate::DestackFormatContext;
use crate::format::analysis::{
    argument_is_collection_literal, argument_is_trivial_unannotated_non_lambda_value,
};
use crate::format::chain::{
    argument_value_id_if_present, is_expression_chain, transparent_inner_expression,
};
use crate::format::expression::is_complex_argument;
use crate::format::tree::{argument_is_block_callback, has_multiline_jsx_argument};
use destack_ast::{Argument, Declaration, Expression, FunctionKind, LocalNodeId, TokenType};

/// Return whether one span intersects any line comment token.
fn span_has_line_comment(ctx: &DestackFormatContext<'_>, span: destack_source::Span) -> bool {
    if span.start >= span.end {
        return false;
    }

    let line_comment_spans = &ctx.line_comment_spans;
    if line_comment_spans.is_empty() {
        return false;
    }

    let first_relevant_index =
        line_comment_spans.partition_point(|comment_span| comment_span.end < span.start);
    for comment_span in &line_comment_spans[first_relevant_index..] {
        if comment_span.start > span.end {
            break;
        }

        if span.intersects(*comment_span) {
            return true;
        }
    }

    false
}

/// Compute call argument layout facts for one call expression.
pub(crate) fn call_argument_layout_facts(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> (bool, bool, bool, bool, bool, bool, usize, usize, bool, bool) {
    let has_call_infix_annotations = ctx.has_non_blank_infix_annotation(call_node_id);

    if dynamic_arguments.is_empty() {
        return (
            has_call_infix_annotations,
            false,
            false,
            true,
            true,
            false,
            0,
            0,
            false,
            false,
        );
    }

    let mut has_any_argument_annotation = false;
    let mut has_line_comment_annotations = false;
    let mut all_single_line_and_unannotated = true;
    let mut all_compact_simple_unannotated = true;
    let mut arrow_argument_count = 0usize;
    let mut function_argument_count = 0usize;
    let mut trailing_collection_argument = false;
    let mut has_complex_non_callback_argument = false;
    let mut has_boundary_comments = false;

    let call_span = ctx.span(call_node_id);
    let last_argument_index = dynamic_arguments.len().saturating_sub(1);

    // scan each argument once and collect layout flags
    for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
        let argument_span = ctx.span(argument_id);

        // boundary comment seams
        if index == 0
            && call_span.file == argument_span.file
            && call_span.start < argument_span.start
            && let Some(open_parenthesis_token) =
                ctx.previous_non_whitespace_token_before_span(argument_span)
            && open_parenthesis_token.token.ty == TokenType::OpenParenthesis
            && open_parenthesis_token.span.end < argument_span.start
        {
            let leading_boundary_span = destack_source::Span::new(
                argument_span.file,
                open_parenthesis_token.span.end,
                argument_span.start,
            );
            has_boundary_comments |= span_has_line_comment(ctx, leading_boundary_span);
        }
        if index > 0 && !has_boundary_comments {
            let previous_argument_span = ctx.span(dynamic_arguments[index - 1]);
            if let Some(between_span) = previous_argument_span.gap_to(argument_span) {
                has_boundary_comments = span_has_line_comment(ctx, between_span);
            }
        }

        // layout decisions only treat non blank annotations as comment signals
        let has_annotation = ctx.has_non_blank_annotation(argument_id);
        let has_newline = ctx.node_has_newline(argument_id);
        let is_last_argument = index == last_argument_index;
        if has_annotation {
            has_any_argument_annotation = true;
            if !has_line_comment_annotations
                && ctx.argument_has_line_comment_annotation(argument_id)
            {
                has_line_comment_annotations = true;
            }
        }

        let is_single_line_and_unannotated = !has_annotation && !has_newline;
        let should_check_compact_simple_unannotated =
            is_single_line_and_unannotated && all_compact_simple_unannotated;
        let compact_simple_unannotated = should_check_compact_simple_unannotated
            .then(|| argument_is_trivial_unannotated_non_lambda_value(ctx, argument_id));

        all_single_line_and_unannotated &= is_single_line_and_unannotated;
        if all_compact_simple_unannotated {
            all_compact_simple_unannotated = compact_simple_unannotated.unwrap_or(false);
        }

        let argument = ctx.tree.get(argument_id);
        let Some(value_id) = argument_value_id_if_present(ctx.tree, argument_id) else {
            continue;
        };
        let value_id = transparent_inner_expression(ctx, value_id);
        let value = ctx.tree.get(value_id);

        if is_last_argument {
            trailing_collection_argument = matches!(
                value,
                Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
            );
        }

        let (is_lambda_argument, is_function_argument, _is_block_callback) = match value {
            Expression::Declaration(declaration_id) => match ctx.tree.get(*declaration_id) {
                Declaration::Function {
                    signature, body, ..
                } => {
                    let is_lambda_argument = signature.kind == FunctionKind::Lambda;
                    let is_function_argument = !is_lambda_argument;
                    let is_block_callback = is_lambda_argument
                        && body.is_some_and(|body_id| {
                            let body_id = transparent_inner_expression(ctx, body_id);
                            matches!(ctx.tree.get(body_id), Expression::Block(_))
                        });
                    (is_lambda_argument, is_function_argument, is_block_callback)
                }
                _ => (false, false, false),
            },
            _ => (false, false, false),
        };

        if is_lambda_argument {
            arrow_argument_count += 1;
        }
        if is_function_argument {
            function_argument_count += 1;
        }

        if is_lambda_argument || is_function_argument {
            continue;
        }

        if !has_complex_non_callback_argument
            && !matches!(value, Expression::TreeExpression { .. })
            && is_complex_argument(ctx.tree, argument)
        {
            has_complex_non_callback_argument = true;
        }
    }

    // trailing boundary seam
    if !has_boundary_comments && let Some(last_argument_id) = dynamic_arguments.last().copied() {
        let last_argument_span = ctx.span(last_argument_id);
        if call_span.file == last_argument_span.file && last_argument_span.end < call_span.end {
            let token_index = ctx
                .tokens
                .partition_point(|token| token.span.start < last_argument_span.end);
            let close_parenthesis_start = ctx.tokens[token_index..]
                .iter()
                .find(|token| token.token.ty == TokenType::CloseParenthesis)
                .map(|token| token.span.start)
                .unwrap_or(call_span.end);
            let boundary_end = close_parenthesis_start.min(call_span.end);
            let boundary_span = destack_source::Span::new(
                last_argument_span.file,
                last_argument_span.end,
                boundary_end,
            );
            has_boundary_comments = span_has_line_comment(ctx, boundary_span);
        }
    }

    (
        has_call_infix_annotations,
        has_boundary_comments,
        has_any_argument_annotation,
        all_single_line_and_unannotated,
        all_compact_simple_unannotated,
        has_line_comment_annotations,
        arrow_argument_count,
        function_argument_count,
        has_complex_non_callback_argument,
        trailing_collection_argument,
    )
}

/// Return whether one argument has callback-blocking line or multiline prefix annotations.
pub(crate) fn argument_has_callback_blocking_comment_annotation(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.argument_has_line_comment_annotation(argument_id)
        || ctx.argument_has_prefix_line_comment_annotation(argument_id)
        || (ctx.node_has_newline(argument_id)
            && ctx.argument_has_prefix_annotation_signal(argument_id))
}

// call argument layout thresholds
const MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT: usize = 2;

/// Resolve chain call argument force-expand state.
pub(crate) fn chain_call_argument_force_expand(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let (
        has_call_infix_annotations,
        _has_boundary_comments,
        _has_any_argument_annotation,
        _all_single_line_and_unannotated,
        _all_compact_simple_unannotated,
        has_line_comment_annotations,
        arrow_argument_count,
        function_argument_count,
        has_complex_non_callback_argument,
        trailing_collection_argument,
    ) = call_argument_layout_facts(ctx, call_node_id, dynamic_arguments);
    if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];
        let has_collection_source_comment = argument_is_collection_literal(ctx, argument_id)
            && ctx.has_comment(ctx.span(argument_id));

        has_multiline_jsx_argument(ctx.tree, dynamic_arguments)
            || has_call_infix_annotations
            || ctx.argument_has_line_comment_annotation(argument_id)
            || ctx.argument_has_prefix_line_comment_annotation(argument_id)
            || has_collection_source_comment
            || single_argument_requires_expanded_list(ctx, dynamic_arguments)
    } else {
        let has_callback_prefix = dynamic_arguments[..dynamic_arguments.len().saturating_sub(1)]
            .iter()
            .copied()
            .any(|argument_id| argument_is_block_callback(ctx, argument_id));

        has_multiline_jsx_argument(ctx.tree, dynamic_arguments)
            || has_call_infix_annotations
            || has_line_comment_annotations
            || has_complex_non_callback_argument
            || arrow_argument_count >= MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT
            || function_argument_count >= MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT
            || (trailing_collection_argument && has_callback_prefix)
    }
}

/// Return whether a single static argument call should force expansion.
pub(crate) fn call_force_expand_single_multiline_with_static_arguments(
    _ctx: &DestackFormatContext<'_>,
    _call_node_id: LocalNodeId<Expression>,
    _dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    false
}

/// Return whether a single collection argument should expand for type binary callees.
pub(crate) fn call_force_expand_single_collection_for_type_binary_callee(
    _ctx: &DestackFormatContext<'_>,
    _call_node_id: LocalNodeId<Expression>,
    _dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    false
}

/// Return whether a single argument call should force expanded list layout.
pub(crate) fn single_argument_requires_expanded_list(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    // only single argument calls can use this path
    if dynamic_arguments.len() != 1 {
        return false;
    }

    // require a chain shaped argument value
    let argument_id = dynamic_arguments[0];
    let Some(raw_value_id) = argument_value_id_if_present(ctx.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(ctx, raw_value_id);
    let is_chain_layout_candidate = is_expression_chain(ctx.tree, value_id)
        || matches!(ctx.tree.get(value_id), Expression::Path { path, .. } if path.segments.len() > 1)
        || matches!(
            ctx.tree.get(value_id),
            Expression::Call { left, .. } | Expression::Instantiation { left, .. }
                if matches!(ctx.tree.get(*left), Expression::Path { path, .. } if path.segments.len() > 1)
        );
    if !is_chain_layout_candidate {
        return false;
    }

    // force expand only from non-boundary annotation signals on the argument value path
    let has_annotation_signal = ctx.has_non_blank_non_boundary_annotation(argument_id)
        || ctx.has_non_blank_non_boundary_annotation(raw_value_id)
        || ctx.has_non_blank_non_boundary_annotation(value_id);
    let has_boundary_signal = ctx.has_boundary_comment_annotation(argument_id)
        || ctx.has_boundary_comment_annotation(raw_value_id)
        || ctx.has_boundary_comment_annotation(value_id);

    has_annotation_signal || has_boundary_signal
}
