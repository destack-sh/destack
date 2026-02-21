use super::super::{
    ChainExpression, DestackFormatContext, PostfixPosition, chain_call_can_expand_in_head,
    is_numeric_index, is_simple_chain_operation,
};
use super::annotation::chain_node_has_non_inline_annotation;

/// Split off simple head operations that should stay with the base.
pub(crate) fn split_chain_head_operations(
    context: &DestackFormatContext<'_>,
    base_has_leading_call_like: bool,
    operations: &[ChainExpression],
    allow_wide_head: bool,
    is_conditional_branch: bool,
) -> usize {
    // nothing to split when there are no operations
    if operations.is_empty() {
        return 0;
    }

    // detect whether the chain starts with calls or numeric indexes
    let first_is_call_or_numeric_index = match operations.first() {
        Some(ChainExpression::Call { .. }) => true,
        Some(ChainExpression::Instantiation { .. }) => true,
        Some(ChainExpression::Index { index, .. }) => is_numeric_index(context, index),
        _ => false,
    };
    let starts_with_member = matches!(operations.first(), Some(ChainExpression::Member { .. }));
    let has_call_like_tail = operations.iter().any(|operation| {
        matches!(
            operation,
            ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
        )
    });
    let has_index_tail = operations
        .iter()
        .any(|operation| matches!(operation, ChainExpression::Index { .. }));

    // keep index-heavy member chains stable: they should break before member hops
    if starts_with_member && has_index_tail && !allow_wide_head {
        return 0;
    }

    let cap_member_promotion_before_call_tail =
        starts_with_member && has_call_like_tail && !allow_wide_head;

    // accumulate promotable simple operations
    let mut head_ops_count = 0usize;
    let mut index = 0usize;

    while index < operations.len() {
        // keep member-leading fluent call chains from over-promoting the head
        if cap_member_promotion_before_call_tail && head_ops_count > 0 {
            break;
        }

        let operation = &operations[index];
        if matches!(operation, ChainExpression::Maybe { .. }) {
            break;
        }

        // avoid splitting a member from its immediate call or index
        let next_operation = operations.get(index + 1);
        let next_is_call_or_index = matches!(
            next_operation,
            Some(
                ChainExpression::Call { .. }
                    | ChainExpression::Index { .. }
                    | ChainExpression::Instantiation { .. }
            )
        );

        if matches!(operation, ChainExpression::Member { .. }) && next_is_call_or_index {
            // only member-call pairs have promotion candidates
            let Some(next_operation) = next_operation else {
                break;
            };

            let allow_single_member_call_pair_after_call_like_base = base_has_leading_call_like
                && index == 0
                && operations.len() == 2
                && matches!(
                    next_operation,
                    ChainExpression::Call {
                        node_id,
                        dynamic_arguments,
                        ..
                    } if dynamic_arguments.len() == 1
                        && !context.node_has_newline(*node_id)
                        && !chain_node_has_non_inline_annotation(context, *node_id)
                );

            // keep call-root chains one hop per line:
            // once a chain starts with a call-like base or operation, do not absorb following
            // member-call pairs into the head because that forces inner-call breaks instead of
            // dot breaks
            if first_is_call_or_numeric_index
                || (base_has_leading_call_like
                    && !allow_single_member_call_pair_after_call_like_base)
            {
                break;
            }

            // only promote the pair when both operations are simple
            let is_single_member_call_pair = index == 0
                && operations.len() == 2
                && matches!(
                    next_operation,
                    ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
                );
            let allow_single_member_call_pair_promotion =
                allow_wide_head && is_single_member_call_pair;

            // when a chain has more member hops after a member + call pair:
            // keep fluent chains one hop per line
            let has_later_member_hop = operations.get(index + 2..).is_some_and(|tail| {
                tail.iter()
                    .any(|op| matches!(op, ChainExpression::Member { .. }))
            });
            if has_later_member_hop && !is_conditional_branch {
                break;
            }

            let next_call_can_expand = matches!(
                next_operation,
                ChainExpression::Call {
                    node_id,
                    static_arguments,
                    dynamic_arguments,
                    ..
                } if chain_call_can_expand_in_head(
                    context,
                    *node_id,
                    static_arguments,
                    dynamic_arguments
                )
            );
            let next_is_promotable_single_argument_call = matches!(
                next_operation,
                ChainExpression::Call {
                    node_id,
                    dynamic_arguments,
                    ..
                } if dynamic_arguments.len() == 1
                    && (allow_wide_head || operations.len() == 2)
                    && !chain_node_has_non_inline_annotation(context, *node_id)
            );
            let should_keep_member_call_pair_split = !allow_wide_head && next_call_can_expand;
            if should_keep_member_call_pair_split {
                break;
            }

            if (!is_simple_chain_operation(context, operation)
                && !allow_single_member_call_pair_promotion)
                || (!is_simple_chain_operation(context, next_operation)
                    && !next_call_can_expand
                    && !next_is_promotable_single_argument_call
                    && !allow_single_member_call_pair_promotion)
            {
                break;
            }

            // keep fluent `foo().bar().baz(...)` call ladders one call per line
            // when the chain already starts with a direct call
            let next_is_empty_call = matches!(
                next_operation,
                ChainExpression::Call {
                    static_arguments,
                    dynamic_arguments,
                    ..
                } if static_arguments
                    .as_ref()
                    .is_none_or(|arguments| arguments.is_empty())
                    && dynamic_arguments.is_empty()
            );
            if base_has_leading_call_like && next_is_empty_call {
                break;
            }

            head_ops_count += 2;
            index += 2;
            continue;
        }

        let previous_op_is_direct_call =
            operations
                .get(index.saturating_sub(1))
                .is_some_and(|operation| {
                    matches!(
                        operation,
                        ChainExpression::Call {
                            position: PostfixPosition::Direct,
                            ..
                        }
                    )
                });

        // only promote simple operations
        if !is_simple_chain_operation(context, operation) {
            break;
        }

        let is_call = matches!(
            operation,
            ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
        );
        let is_numeric_index_op = matches!(
            operation,
            ChainExpression::Index { index, .. } if is_numeric_index(context, index)
        );

        // when the chain starts with calls, keep only call-like head operations
        if first_is_call_or_numeric_index && !(is_call || is_numeric_index_op) {
            break;
        }

        // when the chain starts with members, stop before the first call
        // allow direct call tails after a promoted call: `foo(...)(...)`
        if !first_is_call_or_numeric_index && is_call && !previous_op_is_direct_call {
            break;
        }

        head_ops_count += 1;
        index += 1;
    }

    head_ops_count
}
