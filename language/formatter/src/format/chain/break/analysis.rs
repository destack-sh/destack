use super::super::{
    AnnotationPosition, DestackFormatContext, Expression, LocalNodeId, PostfixPosition,
    argument_forces_multiline, argument_is_function_expression,
    argument_is_inline_closure_cast_object, chain_head_id,
};
use super::annotation::{
    chain_annotation_is_inline_non_breaking, chain_annotation_is_internal_call_argument_infix,
    chain_node_has_breaking_annotation,
};
use super::intervening::{chain_has_intervening_break_or_comment, chain_has_intervening_comment};
use super::overflow::chain_overflows_in_type_binary_left;
use super::path::expression_is_in_conditional_branch;

/// Summarize the complexity of a call within a chain.
pub(crate) struct ChainCallSummary {
    pub(crate) has_multiline_argument: bool,
}

/// Collect break-relevant signals for one chain.
pub(crate) struct ChainBreakAnalysis {
    pub(crate) should_break: bool,
    pub(crate) call_summaries: Vec<ChainCallSummary>,
    pub(crate) has_chain_intervening_trivia: bool,
}

/// Build call summaries for a chain in source order.
pub(crate) fn summarize_chain_calls(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> Vec<ChainCallSummary> {
    let mut summaries = Vec::new();

    for expression_id in chain {
        let Expression::Call {
            static_arguments,
            dynamic_arguments,
            ..
        } = context.tree.get(*expression_id)
        else {
            continue;
        };

        // collect per-call signals
        let has_multiline_argument = dynamic_arguments
            .iter()
            .copied()
            .any(|argument_id| argument_forces_multiline(context, argument_id))
            || static_arguments.as_ref().is_some_and(|arguments| {
                arguments
                    .iter()
                    .copied()
                    .any(|argument_id| argument_forces_multiline(context, argument_id))
            });
        summaries.push(ChainCallSummary {
            has_multiline_argument,
        });
    }

    summaries
}

/// Build chain break signals once so callers can reuse them.
pub(crate) fn analyze_chain_break(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> ChainBreakAnalysis {
    if chain.is_empty() {
        return ChainBreakAnalysis {
            should_break: false,
            call_summaries: Vec::new(),
            has_chain_intervening_trivia: false,
        };
    }

    let chain_root = chain[0];
    let chain_tail = chain[chain.len() - 1];
    let chain_head = chain_head_id(context.tree, chain_root);
    let call_summaries = summarize_chain_calls(context, chain);
    let has_chain_intervening_trivia = chain_has_intervening_break_or_comment(context, chain);
    let has_chain_intervening_comment = chain_has_intervening_comment(context, chain);
    let root_has_path_tail_segments = matches!(
        context.tree.get(chain_root),
        Expression::Path { path, .. } if path.segments.len() > 1
    );
    let has_optional_tail = chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Maybe { .. }
                | Expression::Call {
                    position: PostfixPosition::Indirect,
                    ..
                }
        )
    });
    let has_member_access = chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
    });

    // root prefix annotations are statement-level concerns:
    // root non-prefix annotation handling stays in `head_has_non_prefix_breaking_annotation`
    let has_chain_node_annotations = chain
        .iter()
        .copied()
        .skip(1)
        .any(|expression_id| chain_node_has_breaking_annotation(context, expression_id));
    let head_has_non_prefix_breaking_annotation = context
        .visit_annotations(chain_head, |annotations| {
            annotations.iter().any(|annotation_id| {
                if chain_annotation_is_inline_non_breaking(context, *annotation_id)
                    || chain_annotation_is_internal_call_argument_infix(
                        context,
                        chain_head,
                        *annotation_id,
                    )
                {
                    return false;
                }

                let position = context.annotation(*annotation_id).position();
                if matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return false;
                }

                matches!(
                    position,
                    AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                )
            })
        })
        .unwrap_or(false);
    let has_chain_annotations =
        has_chain_node_annotations || head_has_non_prefix_breaking_annotation;
    let should_break_for_annotation_or_trivia =
        has_chain_annotations || (has_chain_intervening_comment && has_member_access);
    if should_break_for_annotation_or_trivia {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
        };
    }

    let should_break_for_path_optional_tail =
        has_chain_intervening_trivia && root_has_path_tail_segments && has_optional_tail;
    if should_break_for_path_optional_tail {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
        };
    }

    let has_direct_curried_call_pair = chain.windows(2).any(|pair| {
        pair.iter().all(|expression_id| {
            matches!(
                context.tree.get(*expression_id),
                Expression::Call {
                    position: PostfixPosition::Direct,
                    ..
                }
            )
        })
    });
    let should_break_for_curried_call_intervening_trivia =
        has_chain_intervening_trivia && has_direct_curried_call_pair;
    if should_break_for_curried_call_intervening_trivia {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
        };
    }

    let has_direct_call_with_inline_closure_cast_object_argument =
        chain.iter().copied().any(|expression_id| {
            let Expression::Call {
                position: PostfixPosition::Direct,
                dynamic_arguments,
                ..
            } = context.tree.get(expression_id)
            else {
                return false;
            };

            dynamic_arguments
                .iter()
                .copied()
                .any(|argument_id| argument_is_inline_closure_cast_object(context, argument_id))
        });
    if has_direct_call_with_inline_closure_cast_object_argument {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
        };
    }

    // chains with no calls stay inline unless they overflow
    if call_summaries.is_empty() {
        return ChainBreakAnalysis {
            should_break: false,
            call_summaries,
            has_chain_intervening_trivia,
        };
    }

    let calls_count = call_summaries.len();
    let has_multiline_call = call_summaries
        .iter()
        .any(|summary| summary.has_multiline_argument);
    let chain_is_in_conditional_branch = expression_is_in_conditional_branch(context, chain_tail)
        || expression_is_in_conditional_branch(context, chain_root);

    if calls_count > 1 && has_multiline_call && !chain_is_in_conditional_branch {
        return ChainBreakAnalysis {
            should_break: true,
            call_summaries,
            has_chain_intervening_trivia,
        };
    }

    // single-call chains should prefer the regular inline fit decision
    if calls_count == 1 {
        return ChainBreakAnalysis {
            should_break: false,
            call_summaries,
            has_chain_intervening_trivia,
        };
    }

    let should_break = chain_overflows_in_type_binary_left(context, chain_tail);

    ChainBreakAnalysis {
        should_break,
        call_summaries,
        has_chain_intervening_trivia,
    }
}

/// Decide whether a chain should proactively break across lines.
pub(crate) fn should_break_chain(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    analyze_chain_break(context, chain).should_break
}

/// Return whether a non-head call in a chain takes a non-lambda function argument.
pub(crate) fn chain_has_nonhead_nonlambda_function_call_argument(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    let mut call_index = 0usize;

    for expression_id in chain {
        let Expression::Call {
            dynamic_arguments, ..
        } = context.tree.get(*expression_id)
        else {
            continue;
        };

        if call_index > 0
            && dynamic_arguments
                .iter()
                .copied()
                .any(|argument_id| argument_is_function_expression(context, argument_id))
        {
            return true;
        }

        call_index += 1;
    }

    false
}
