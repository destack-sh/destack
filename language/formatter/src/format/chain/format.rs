use super::normalize::{ChainLayoutPlan, plan_chain_layout};
use super::policy::{
    ChainRenderDecision, ChainRenderInputOptions, build_chain_render_inputs, decide_chain_render,
};
use super::*;
use destack_fir::write;

/// Return whether one call operation should emit its prefix annotations.
fn call_operation_should_emit_prefix_annotations(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = context.tree.get(call_node_id) else {
        return context.has_prefix_annotation(call_node_id);
    };

    if !context.has_prefix_annotation(call_node_id) {
        return false;
    }

    if !context.has_prefix_annotation(*left) {
        return true;
    }

    let left_span = context.span(*left);
    let has_leading_prefix_before_left = context
        .visit_annotations(call_node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                if !matches!(
                    annotation.position(),
                    AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
                ) {
                    return false;
                }

                let annotation_span = context.annotation_span(*annotation_id);
                annotation_span.start < left_span.start
            })
        })
        .unwrap_or(false);

    !has_leading_prefix_before_left
}

/// Return whether one chain operation is an index access.
fn chain_operation_is_index(operation: &ChainExpression) -> bool {
    matches!(operation, ChainExpression::Index { .. })
}

/// Return the first operation of the first grouped line.
fn first_grouped_line_operation(
    lines: &[SmallVec<[ChainExpression; 2]>],
) -> Option<&ChainExpression> {
    lines.first().and_then(|line| line.first())
}

/// Format static arguments for chain operations that may need relational spacing.
fn format_chain_static_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_arguments: &[LocalNodeId<Argument>],
    next_operation: Option<&ChainExpression>,
) -> FormatResult<()> {
    if next_operation.is_some_and(chain_operation_is_index) {
        return format_static_argument_list_with_relational_spacing(f, static_arguments);
    }

    format_static_argument_list(f, static_arguments)
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
        should_break: chain_should_break,
        has_calls: chain_has_calls,
        in_template_literal_interpolation,
        instantiation_prefix_wrap_body_ops,
    } = plan;

    // indent chain lines consistently, even in assignment rhs positions
    let should_indent_chain = true;

    // inline variant keeps everything on one line when it fits
    let format_inline = format_with(|f| {
        format_chain_base(
            f,
            node_id,
            &base,
            &lines,
            instantiation_prefix_wrap_body_ops,
        )?;
        for line in &lines {
            format_chain_expression_line(f, node_id, line)?;
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
            format_chain_base(
                f,
                node_id,
                &base,
                &lines,
                instantiation_prefix_wrap_body_ops,
            )?;
            // indent chained entries so each operation sits on its own line
            // use indent with manual line breaks instead of block_indent to avoid trailing newline
            // this keeps semicolons on the same line as the last chain element
            if !lines.is_empty() {
                let format_lines = format_with(|f| {
                    // each chain line renders in isolation to mirror prettier style
                    for (line_index, line) in lines.iter().enumerate() {
                        if line_index == 0
                            || !chain_line_starts_with_block_prefix_annotation(f.context(), line)
                        {
                            write!(f, [hard_line_break()])?;
                        }
                        format_chain_expression_line(f, node_id, line)?;
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

    let render_options = ChainRenderInputOptions {
        node_id,
        chain_should_break,
        chain_has_calls,
        in_template_literal_interpolation,
    };
    let render_inputs = build_chain_render_inputs(f.context(), &base, &lines, render_options);

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
    formatted_root_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    instantiation_prefix_wrap_body_ops: Option<usize>,
) -> FormatResult<()> {
    format_chain_base_content(
        f,
        formatted_root_id,
        base,
        lines,
        instantiation_prefix_wrap_body_ops,
    )
}

/// Format the unwrapped base segment of a chain.
fn format_chain_base_content<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    instantiation_prefix_wrap_body_ops: Option<usize>,
) -> FormatResult<()> {
    let mut has_open_prefix_wrap = false;
    let mut has_closed_prefix_wrap = false;
    if instantiation_prefix_wrap_body_ops.is_some() {
        write!(f, [token("(")])?;
        has_open_prefix_wrap = true;
    }

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
                let next_operation = base
                    .body
                    .first()
                    .or_else(|| first_grouped_line_operation(lines));
                format_chain_static_argument_list(f, arguments, next_operation)?;
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

    if instantiation_prefix_wrap_body_ops == Some(0) {
        write!(f, [token(")")])?;
        has_closed_prefix_wrap = true;
    }

    for (index, op) in base.body.iter().enumerate() {
        let next_operation = base
            .body
            .get(index + 1)
            .or_else(|| first_grouped_line_operation(lines));
        format_chain_expression(f, formatted_root_id, op, next_operation)?;
        if !has_closed_prefix_wrap && instantiation_prefix_wrap_body_ops == Some(index + 1) {
            write!(f, [token(")")])?;
            has_closed_prefix_wrap = true;
        }
    }

    if has_open_prefix_wrap && !has_closed_prefix_wrap {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one chained operation.
fn format_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    op: &ChainExpression,
    next_operation: Option<&ChainExpression>,
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
        | ChainExpression::Must { node_id, .. } => {
            let should_emit_prefix_annotations = match op {
                ChainExpression::Call { node_id, .. } => {
                    call_operation_should_emit_prefix_annotations(f.context(), *node_id)
                }
                _ => true,
            };
            (*node_id, should_emit_prefix_annotations, true)
        }
    };
    let call_or_new_handles_empty_infix = matches!(
        op,
        ChainExpression::Call {
            node_id,
            dynamic_arguments,
            ..
        } if dynamic_arguments.is_empty() && f.context().has_infix_annotation(*node_id)
    );
    let emit_prefix_annotations = emit_prefix_annotations && node_id != formatted_root_id;
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
                format_chain_static_argument_list(f, arguments, next_operation)?;
            }
        }
        ChainExpression::Instantiation {
            static_arguments, ..
        } => {
            format_chain_static_argument_list(f, static_arguments, next_operation)?;
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
            format_call_arguments(f, *call_node_id, dynamic_arguments)?;
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
        if call_or_new_handles_empty_infix {
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        } else {
            write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        }
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
    formatted_root_id: LocalNodeId<Expression>,
    ops: &[ChainExpression],
) -> FormatResult<()> {
    for (index, op) in ops.iter().enumerate() {
        let next_operation = ops.get(index + 1);
        format_chain_expression(f, formatted_root_id, op, next_operation)?;
    }
    Ok(())
}
