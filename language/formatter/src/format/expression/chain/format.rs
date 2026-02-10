use super::*;
use destack_fir::write;

/// The root-derived base and synthetic operations for chain formatting.
struct ChainRootParts {
    head: ChainExpressionBaseHead,
    operations: Vec<ChainExpression>,
    deferred_boundary_comments: Vec<String>,
}

/// Normalized chain inputs before layout scoring.
struct NormalizedChainLayout {
    chain: Vec<LocalNodeId<Expression>>,
    root_id: LocalNodeId<Expression>,
    base: ChainExpressionBase,
    body: Vec<ChainExpression>,
    deferred_path_boundary_comments: Vec<String>,
}

/// Planned chain layout used by render-only formatting.
struct ChainLayoutPlan {
    base: ChainExpressionBase,
    lines: Vec<SmallVec<[ChainExpression; 2]>>,
    deferred_path_boundary_comments: Vec<String>,
    should_break: bool,
    has_calls: bool,
    in_template_literal_interpolation: bool,
}

/// Store one-pass operation facts used by chain render decisions.
#[derive(Default)]
struct ChainOperationFacts {
    inline_chain_len: usize,
    has_call_with_dynamic_arguments: bool,
    has_optional_chain_operation: bool,
    has_multiline_dynamic_call_argument: bool,
}

/// Store normalized render inputs for chain decision making.
struct ChainRenderInputs {
    chain_should_break: bool,
    chain_has_calls: bool,
    in_template_literal_interpolation: bool,
    has_deferred_path_boundary_comments: bool,
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
enum ChainRenderDecision {
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

/// Build normalized chain render inputs once from layout plan and source context.
fn build_chain_render_inputs(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    deferred_path_boundary_comments: &[String],
    chain_should_break: bool,
    chain_has_calls: bool,
    in_template_literal_interpolation: bool,
) -> ChainRenderInputs {
    let line_width = usize::from(context.options.line_width);
    let chain_span = context.get_span(node_id);
    let is_chain_call_like_argument = is_call_like_argument(context, node_id);
    let is_chain_conditional_branch = expression_is_in_conditional_branch(context, node_id);
    let assignment_like_width = if is_chain_call_like_argument
        || in_template_literal_interpolation
        || is_chain_conditional_branch
        || !chain_has_calls
    {
        None
    } else {
        assignment_like_remaining_width(context, node_id)
    };
    let inline_budget = if is_chain_call_like_argument
        || in_template_literal_interpolation
        || is_chain_conditional_branch
    {
        line_width
    } else if chain_has_calls {
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
        chain_should_break,
        chain_has_calls,
        in_template_literal_interpolation,
        has_deferred_path_boundary_comments: !deferred_path_boundary_comments.is_empty(),
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
fn decide_chain_render(inputs: &ChainRenderInputs, lines_len: usize) -> ChainRenderDecision {
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
        let mut segments = path.segments.clone().into_iter();
        let static_arguments = static_arguments.clone();
        let Some(first_segment) = segments.next() else {
            return Err(FormatError::SyntaxError {
                message: "path chain root must contain at least one segment",
            });
        };
        let remaining_segments: Vec<StringId> = segments.collect();
        deferred_boundary_comments =
            path_deferred_boundary_line_comments(context, root_id, path.segments.len());

        let emit_postfix_on_tail = !remaining_segments.is_empty()
            && path_postfix_annotations_emit_on_tail(context, root_id, path.segments.len());

        // path base keeps static arguments only when there is no synthetic tail
        let base_static_arguments = if remaining_segments.is_empty() {
            static_arguments.clone()
        } else {
            None
        };
        head = ChainExpressionBaseHead::Path {
            node_id: root_id,
            segment: first_segment,
            static_arguments: base_static_arguments,
            emit_postfix_annotations: (remaining_segments.is_empty() || !emit_postfix_on_tail)
                && deferred_boundary_comments.is_empty(),
        };

        // append synthetic member operations for remaining path segments
        let tail_len = remaining_segments.len();
        for (index, segment) in remaining_segments.into_iter().enumerate() {
            let is_last = tail_len != 0 && index + 1 == tail_len;
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
fn plan_chain_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<ChainLayoutPlan> {
    context.increment_counter("profile.chain.layout.builds", 1);

    let mut normalized = normalize_chain_layout(context, node_id)?;
    let ChainBreakAnalysis {
        should_break: break_analysis_should_break,
        call_summaries: chain_call_summaries,
        has_chain_intervening_trivia,
        has_path_tail_deferred_empty_call_boundary_comment,
    } = analyze_chain_break(context, &normalized.chain);
    let mut should_break = break_analysis_should_break;
    let has_multiline_nonhead_call = chain_call_summaries
        .iter()
        .skip(1)
        .any(|summary| summary.has_multiline_argument);
    let has_nonhead_nonlambda_function_call_argument =
        chain_has_nonhead_nonlambda_function_call_argument(context, &normalized.chain);

    // member operations with non inline annotations should keep one operation per line
    let member_has_non_inline_annotation = normalized.body.iter().any(|operation| {
        let ChainExpression::Member { node_id, .. } = operation else {
            return false;
        };
        chain_node_has_breaking_annotation(context, *node_id)
    });
    if member_has_non_inline_annotation {
        should_break = true;
    }
    let root_has_line_postfix_boundary_comment =
        expression_has_line_postfix_boundary_comment(context, normalized.root_id);
    let first_member_has_line_postfix_boundary_comment =
        normalized.body.first().is_some_and(|operation| {
            let ChainExpression::Member { node_id, .. } = operation else {
                return false;
            };
            expression_has_line_postfix_boundary_comment(context, *node_id)
        });
    let starts_with_member_operation = matches!(
        normalized.body.first(),
        Some(ChainExpression::Member { .. })
    );
    let should_avoid_head_promotion_for_boundary_comment =
        first_member_has_line_postfix_boundary_comment
            || (root_has_line_postfix_boundary_comment && starts_with_member_operation);
    if should_avoid_head_promotion_for_boundary_comment {
        should_break = true;
    }

    // keep a small head group with the base for prettier style chains
    let base_len = chain_base_len(context, &normalized.base);
    let base_has_leading_call_like = match &normalized.base.head {
        ChainExpressionBaseHead::Expression(expression_id) => matches!(
            context.tree.get(*expression_id),
            Expression::Call { .. } | Expression::Instantiation { .. }
        ),
        ChainExpressionBaseHead::Path { .. } => {
            normalized.base.body.first().is_some_and(|operation| {
                matches!(
                    operation,
                    ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
                )
            })
        }
    };
    let in_conditional_branch = expression_is_in_conditional_branch(context, node_id);
    let remaining_width = if is_call_like_argument(context, node_id) || in_conditional_branch {
        None
    } else {
        assignment_like_remaining_width(context, node_id)
    };
    let root_has_annotation = context.has_annotation(normalized.root_id);
    let should_avoid_head_promotion_for_nonhead_callbacks =
        has_nonhead_nonlambda_function_call_argument
            || (has_multiline_nonhead_call && has_chain_intervening_trivia)
            || has_path_tail_deferred_empty_call_boundary_comment
            || root_has_annotation;
    let allow_wide_head = is_call_like_argument(context, node_id);
    let head_ops_count = if should_avoid_head_promotion_for_nonhead_callbacks
        || should_avoid_head_promotion_for_boundary_comment
    {
        0
    } else {
        split_chain_head_operations(
            context,
            base_len,
            base_has_leading_call_like,
            &normalized.body,
            remaining_width,
            allow_wide_head,
        )
    };
    if head_ops_count > 0 {
        let head_ops: Vec<_> = normalized.body.drain(..head_ops_count).collect();
        normalized.base.body.extend(head_ops);
    }

    // group the chain operations into lines
    let mut lines = group_chain_expression_lines(context, normalized.body);

    // keep curried call tails attached to an already promoted direct call head:
    // `foo(...)(...)` should not split between the closing and opening parens
    if normalized.base.body.last().is_some_and(|operation| {
        matches!(
            operation,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            }
        )
    }) && let Some(first_line) = lines.first()
        && first_line.len() == 1
        && let ChainExpression::Call {
            node_id,
            position: PostfixPosition::Direct,
            ..
        } = first_line[0]
        && !chain_node_has_non_inline_annotation(context, node_id)
    {
        let first_line = lines.remove(0);
        normalized.base.body.push(first_line[0].clone());
    }

    // keep short member + call heads compact inside argument positions:
    // `foo.bar.get(...)` should not split before `.get(` by default
    if is_call_like_argument(context, node_id)
        && normalized
            .base
            .body
            .last()
            .is_some_and(|operation| matches!(operation, ChainExpression::Member { .. }))
        && let Some(first_line) = lines.first()
        && first_line.len() == 1
        && let ChainExpression::Call {
            node_id: call_node_id,
            ..
        } = first_line[0]
        && !chain_node_has_non_inline_annotation(context, call_node_id)
    {
        let first_line = lines.remove(0);
        normalized.base.body.push(first_line[0].clone());
    }

    // keep member + call pairs attached in argument chains:
    // `foo.bar.get(...)` should stay together before optional tails
    if is_call_like_argument(context, node_id)
        && normalized
            .base
            .body
            .last()
            .is_some_and(|operation| matches!(operation, ChainExpression::Member { .. }))
        && let Some(first_line) = lines.first()
        && first_line.len() == 2
        && let (
            ChainExpression::Member {
                node_id: member_node_id,
                ..
            },
            ChainExpression::Call {
                node_id: call_node_id,
                ..
            },
        ) = (&first_line[0], &first_line[1])
        && !chain_node_has_non_inline_annotation(context, *member_node_id)
        && !chain_node_has_non_inline_annotation(context, *call_node_id)
    {
        let first_line = lines.remove(0);
        normalized.base.body.extend(first_line);
    }

    // keep short argument chains from fragmenting on their first member hops:
    // `foo.bar` + `.baz(...)` should render as `foo.bar.baz(...)` in argument positions
    if is_call_like_argument(context, node_id)
        && lines.len() >= 2
        && matches!(lines[0].as_slice(), [ChainExpression::Member { .. }])
        && matches!(
            lines[1].first(),
            Some(ChainExpression::Member { .. } | ChainExpression::Call { .. })
        )
    {
        let mut first_line = lines.remove(0);
        let second_line = lines.remove(0);
        first_line.extend(second_line);
        lines.insert(0, first_line);
    }

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

/// Format a member/call/maybe/index chain with prettier-style breaking.
pub(crate) fn format_expression_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let plan = plan_chain_layout(f.context(), node_id)?;
    let ChainLayoutPlan {
        base,
        lines,
        deferred_path_boundary_comments,
        should_break: chain_should_break,
        has_calls: chain_has_calls,
        in_template_literal_interpolation,
    } = plan;

    // indent chain lines consistently, even in assignment rhs positions
    let should_indent_chain = true;

    // inline variant keeps everything on one line when it fits
    let format_inline = format_with(|f| {
        format_chain_base(f, &base)?;
        for line in &lines {
            format_chain_expression_line(f, line)?;
        }
        Ok(())
    });
    // chain variant breaks each operation onto its own line
    let format_chain = format_with(|f| {
        // encourage the parent to break when the chain is complex
        if chain_should_break {
            write!(f, [expand_parent()])?;
        }

        group(&format_with(|f| {
            // always print the base first so indentation aligns subsequent lines
            format_chain_base(f, &base)?;
            // indent chained entries so each operation sits on its own line
            // use indent with manual line breaks instead of block_indent to avoid trailing newline
            // this keeps semicolons on the same line as the last chain element
            if !lines.is_empty() {
                let format_lines = format_with(|f| {
                    for comment in &deferred_path_boundary_comments {
                        write!(f, [hard_line_break(), text(comment.as_str())])?;
                    }

                    // each chain line renders in isolation to mirror prettier style
                    for (line_index, line) in lines.iter().enumerate() {
                        if line_index == 0
                            || !chain_line_starts_with_block_prefix_annotation(f.context(), line)
                        {
                            write!(f, [hard_line_break()])?;
                        }
                        format_chain_expression_line(f, line)?;
                    }
                    Ok(())
                });
                if should_indent_chain {
                    write!(f, [indent(&format_lines)])?;
                } else {
                    // avoid extra indentation when the parent already indents after `=`
                    write!(f, [format_lines])?;
                }
            }
            Ok(())
        }))
        .format(f)
    });

    let render_inputs = build_chain_render_inputs(
        f.context(),
        node_id,
        &base,
        &lines,
        &deferred_path_boundary_comments,
        chain_should_break,
        chain_has_calls,
        in_template_literal_interpolation,
    );

    match decide_chain_render(&render_inputs, lines.len()) {
        ChainRenderDecision::InlineNoCounter => format_inline.format(f),
        ChainRenderDecision::InlineFastPath => {
            f.context()
                .increment_counter("profile.chain.inline.fast_path", 1);
            format_inline.format(f)
        }
        ChainRenderDecision::ForcedBreak => write!(f, [group(&format_chain).should_expand(true)]),
        ChainRenderDecision::ConditionalInline => {
            f.context()
                .increment_counter("profile.chain.conditional.inline", 1);
            format_inline.format(f)
        }
        ChainRenderDecision::MultilineCallArgumentInline => {
            f.context()
                .increment_counter("profile.chain.inline.multiline_call_argument", 1);
            format_inline.format(f)
        }
        ChainRenderDecision::BreakForOverflow => {
            f.context()
                .increment_counter("profile.chain.skip_probe_overflow", 1);
            format_chain.format(f)
        }
        ChainRenderDecision::DeterministicInline => {
            f.context()
                .increment_counter("profile.chain.deterministic.inline", 1);
            format_inline.format(f)
        }
        ChainRenderDecision::DeterministicBreak => {
            f.context()
                .increment_counter("profile.chain.deterministic.chain", 1);
            format_chain.format(f)
        }
    }
}
/// Format the base segment of a chain.
fn format_chain_base<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    base: &ChainExpressionBase,
) -> FormatResult<()> {
    match &base.head {
        ChainExpressionBaseHead::Path {
            node_id,
            segment,
            static_arguments,
            emit_postfix_annotations,
        } => {
            write!(f, [f.context().any_prefix_annotations(*node_id)])?;
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                format_static_argument_list(f, arguments)?;
            }
            if *emit_postfix_annotations {
                write!(f, [f.context().any_infix_or_postfix_annotations(*node_id)])?;
            }
        }
        ChainExpressionBaseHead::Expression(node_id) => {
            let expression = f.context().tree.get(*node_id);
            debug_assert!(
                !matches!(
                    expression,
                    Expression::Member { .. }
                        | Expression::PrivateMember { .. }
                        | Expression::Call { .. }
                        | Expression::Index { .. }
                        | Expression::Maybe { .. }
                ),
                "chain base expression should not be another chain node"
            );
            write_postfix_base_expression(f, *node_id)?;
        }
    }

    for op in &base.body {
        format_chain_expression(f, op)?;
    }

    Ok(())
}

/// Format one chained operation.
fn format_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    op: &ChainExpression,
) -> FormatResult<()> {
    // output any line prefix annotations before the operation
    let (node_id, emit_prefix_annotations, emit_postfix_annotations) = match op {
        ChainExpression::Member {
            node_id,
            emit_prefix_annotations,
            emit_postfix_annotations,
            ..
        } => (
            *node_id,
            *emit_prefix_annotations,
            *emit_postfix_annotations,
        ),
        ChainExpression::Instantiation { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Maybe { node_id, .. }
        | ChainExpression::Must { node_id, .. } => (*node_id, true, true),
    };
    if emit_prefix_annotations {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
    }

    match op {
        ChainExpression::Member {
            node_id,
            segment,
            static_arguments,
            ..
        } => {
            let is_private_hash = member_is_private_hash(f.context(), *node_id);
            write!(f, [token(".")])?;
            if is_private_hash {
                write!(f, [token("#")])?;
            }
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                format_static_argument_list(f, arguments)?;
            }
        }
        ChainExpression::Instantiation {
            static_arguments, ..
        } => {
            format_static_argument_list(f, static_arguments)?;
        }
        ChainExpression::Call {
            node_id: call_node_id,
            position,
            static_arguments,
            dynamic_arguments,
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(arguments) = static_arguments {
                format_static_argument_list(f, arguments)?;
            }
            format_call_dynamic_arguments_with_deferred_comments(
                f,
                *call_node_id,
                dynamic_arguments,
            )?;
        }
        ChainExpression::Index {
            position, index, ..
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(index) = index {
                let should_parenthesize = should_parenthesize_index_expression(f.context(), *index);
                if should_parenthesize {
                    write!(f, [token("["), token("("), *index, token(")"), token("]")])?;
                } else {
                    write!(f, [token("["), *index, token("]")])?;
                }
            } else {
                write!(f, [token("[]")])?;
            }
        }
        ChainExpression::Maybe { position, .. } => match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("?")])?;
            }
        },
        ChainExpression::Must { position, .. } => match position {
            PostfixPosition::Direct => write!(f, [token("!")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("!")])?;
            }
        },
    }

    // output any line postfix annotations after the operation
    if emit_postfix_annotations {
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
    }

    Ok(())
}

/// Decide whether an index expression should be wrapped in parentheses.
pub(crate) fn should_parenthesize_index_expression(
    context: &DestackFormatContext<'_>,
    index_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(context.tree.get(index_id), Expression::Parenthesized { .. }) {
        return false;
    }

    let inner_index_id = transparent_inner_expression(context, index_id);
    matches!(context.tree.get(inner_index_id), Expression::Assign { .. })
}

/// Format all operations for one chain line.
fn format_chain_expression_line<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    ops: &[ChainExpression],
) -> FormatResult<()> {
    for op in ops {
        format_chain_expression(f, op)?;
    }
    Ok(())
}

/// Return whether an expression has a line postfix boundary comment annotation.
fn expression_has_line_postfix_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .with_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.tree.get::<Annotation>(*annotation_id),
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
fn group_chain_expression_lines(
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

    lines
}

type ChainOperationIter = std::iter::Peekable<std::vec::IntoIter<ChainExpression>>;

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
    if should_merge_member_run_with_call(line, iter) {
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
    line: &SmallVec<[ChainExpression; 2]>,
    iter: &ChainOperationIter,
) -> bool {
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
                should_merge = member_run_count >= 2;
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
