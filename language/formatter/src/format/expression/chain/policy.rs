use super::*;

/// Store one-pass operation facts used by chain render decisions.
#[derive(Default)]
struct ChainOperationFacts {
    inline_chain_len: usize,
    has_call_with_dynamic_arguments: bool,
    has_optional_chain_operation: bool,
    has_multiline_dynamic_call_argument: bool,
}

/// Store normalized render inputs for chain decision making.
pub(super) struct ChainRenderInputs {
    pub(super) chain_should_break: bool,
    pub(super) chain_has_calls: bool,
    pub(super) in_template_literal_interpolation: bool,
    pub(super) has_deferred_path_boundary_comments: bool,
    is_chain_call_like_argument: bool,
    is_chain_conditional_branch: bool,
    is_assignment_like_rhs: bool,
    chain_has_source_newline: bool,
    first_line_has_non_empty_dynamic_call: bool,
    inline_budget: usize,
    compact_chain_len: usize,
    operation_facts: ChainOperationFacts,
}

/// Store deterministic chain render decisions.
pub(super) enum ChainRenderDecision {
    InlineNoCounter,
    InlineFastPath,
    ForcedBreak,
    ConditionalInline,
    MultilineCallArgumentInline,
    BreakForOverflow,
    DeterministicInline,
    DeterministicBreak,
}

/// Visit all chain operations in base and grouped line order.
fn for_each_chain_operation(
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    mut visit: impl FnMut(&ChainExpression),
) {
    for operation in &base.body {
        visit(operation);
    }
    for line in lines {
        for operation in line {
            visit(operation);
        }
    }
}

/// Collect operation facts once for chain render decisions.
fn collect_chain_operation_facts(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
) -> ChainOperationFacts {
    let mut facts = ChainOperationFacts {
        inline_chain_len: chain_base_len(context, base),
        ..ChainOperationFacts::default()
    };

    // keep inline length parity with previous behavior:
    // base length plus grouped line operation lengths
    for line in lines {
        for operation in line {
            facts.inline_chain_len = facts
                .inline_chain_len
                .saturating_add(chain_operation_len(context, operation));
        }
    }

    // multiline dynamic argument checks are line-only by design
    for line in lines {
        for operation in line {
            let ChainExpression::Call {
                dynamic_arguments, ..
            } = operation
            else {
                continue;
            };
            if dynamic_arguments.is_empty() {
                continue;
            }
            if !facts.has_multiline_dynamic_call_argument {
                facts.has_multiline_dynamic_call_argument = dynamic_arguments
                    .iter()
                    .any(|argument_id| context.node_has_newline(*argument_id));
            }
        }
    }

    for_each_chain_operation(base, lines, |operation| {
        if matches!(operation, ChainExpression::Maybe { .. }) {
            facts.has_optional_chain_operation = true;
        }

        let ChainExpression::Call {
            dynamic_arguments, ..
        } = operation
        else {
            return;
        };
        if dynamic_arguments.is_empty() {
            return;
        }

        facts.has_call_with_dynamic_arguments = true;
    });

    facts
}

/// Store configurable inputs for chain render planning.
#[derive(Clone, Copy)]
pub(super) struct ChainRenderInputOptions {
    /// The chain expression node id.
    pub(super) node_id: LocalNodeId<Expression>,
    /// Whether chain layout already determined a forced break.
    pub(super) chain_should_break: bool,
    /// Whether the chain contains any call operations.
    pub(super) chain_has_calls: bool,
    /// Whether this chain is in a template literal interpolation.
    pub(super) in_template_literal_interpolation: bool,
    /// Whether deferred path comments must be emitted before chain lines.
    pub(super) has_deferred_path_boundary_comments: bool,
}

/// Build normalized chain render inputs once from layout plan and source context.
pub(super) fn build_chain_render_inputs(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    options: ChainRenderInputOptions,
) -> ChainRenderInputs {
    let line_width = usize::from(context.options.line_width);
    let chain_span = context.get_span(options.node_id);
    let is_chain_call_like_argument = is_call_like_argument(context, options.node_id);
    let is_chain_conditional_branch = expression_is_in_conditional_branch(context, options.node_id);
    let assignment_like_width = if is_chain_call_like_argument
        || options.in_template_literal_interpolation
        || is_chain_conditional_branch
        || !options.chain_has_calls
    {
        None
    } else {
        assignment_like_remaining_width(context, options.node_id)
    };
    let inline_budget = if is_chain_call_like_argument
        || options.in_template_literal_interpolation
        || is_chain_conditional_branch
    {
        line_width
    } else if options.chain_has_calls {
        assignment_like_width.unwrap_or(line_width)
    } else {
        line_width
    };
    let first_line_has_non_empty_dynamic_call = lines.first().is_some_and(|line| {
        line.iter().any(|operation| {
            matches!(
                operation,
                ChainExpression::Call {
                    dynamic_arguments,
                    ..
                } if !dynamic_arguments.is_empty()
            )
        })
    });
    let operation_facts = collect_chain_operation_facts(context, base, lines);

    ChainRenderInputs {
        chain_should_break: options.chain_should_break,
        chain_has_calls: options.chain_has_calls,
        in_template_literal_interpolation: options.in_template_literal_interpolation,
        has_deferred_path_boundary_comments: options.has_deferred_path_boundary_comments,
        is_chain_call_like_argument,
        is_chain_conditional_branch,
        is_assignment_like_rhs: assignment_like_width.is_some(),
        chain_has_source_newline: context.has_newline(chain_span),
        first_line_has_non_empty_dynamic_call,
        inline_budget,
        compact_chain_len: source_min_inline_char_len(context.get_span_str(chain_span)),
        operation_facts,
    }
}

/// Decide how to render a chain from normalized inputs.
pub(super) fn decide_chain_render(
    inputs: &ChainRenderInputs,
    lines_len: usize,
) -> ChainRenderDecision {
    if inputs.in_template_literal_interpolation && !inputs.chain_has_calls {
        return ChainRenderDecision::InlineNoCounter;
    }

    let can_use_inline_fast_path = !inputs.chain_should_break
        && !inputs.has_deferred_path_boundary_comments
        && !inputs.chain_has_source_newline
        && lines_len <= 2
        && inputs.operation_facts.inline_chain_len <= inputs.inline_budget;
    if can_use_inline_fast_path {
        return ChainRenderDecision::InlineFastPath;
    }

    if inputs.chain_should_break {
        return ChainRenderDecision::ForcedBreak;
    }

    let prefer_conditional_inline_chain = inputs.is_chain_conditional_branch
        && !inputs.has_deferred_path_boundary_comments
        && inputs.first_line_has_non_empty_dynamic_call;
    if prefer_conditional_inline_chain {
        return ChainRenderDecision::ConditionalInline;
    }

    let should_avoid_inline_optional_call_chain =
        inputs.operation_facts.has_optional_chain_operation
            && inputs.operation_facts.has_call_with_dynamic_arguments
            && inputs.chain_has_source_newline;
    let can_inline_multiline_call_argument_chain = !inputs.has_deferred_path_boundary_comments
        && !inputs.chain_should_break
        && !should_avoid_inline_optional_call_chain
        && (inputs.is_chain_call_like_argument
            || inputs.in_template_literal_interpolation
            || inputs.is_chain_conditional_branch
            || inputs.is_assignment_like_rhs)
        && inputs.operation_facts.has_multiline_dynamic_call_argument;
    if can_inline_multiline_call_argument_chain {
        return ChainRenderDecision::MultilineCallArgumentInline;
    }

    let should_probe_dynamic_call_overflow = inputs.operation_facts.has_call_with_dynamic_arguments
        && (inputs.is_chain_call_like_argument
            || inputs.in_template_literal_interpolation
            || inputs.is_chain_conditional_branch);
    if inputs.compact_chain_len > inputs.inline_budget && !should_probe_dynamic_call_overflow {
        return ChainRenderDecision::BreakForOverflow;
    }

    let should_inline = !inputs.has_deferred_path_boundary_comments
        && !inputs.chain_should_break
        && !should_avoid_inline_optional_call_chain
        && inputs.operation_facts.inline_chain_len <= inputs.inline_budget;
    if should_inline {
        ChainRenderDecision::DeterministicInline
    } else {
        ChainRenderDecision::DeterministicBreak
    }
}
