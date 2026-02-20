use super::super::{
    ChainExpression, DestackFormatContext, PostfixPosition, chain_call_can_expand_in_head,
    chain_head_operation_len, chain_operation_len, is_numeric_index, is_simple_chain_operation,
};
use super::annotation::chain_node_has_non_inline_annotation;

/// Split off simple head operations that should stay with the base.
pub(crate) fn split_chain_head_operations(
    context: &DestackFormatContext<'_>,
    base_len: usize,
    base_has_leading_call_like: bool,
    operations: &[ChainExpression],
    remaining_width: Option<usize>,
    allow_wide_head: bool,
) -> usize {
    // nothing to split when there are no operations
    if operations.is_empty() {
        return 0;
    }

    // keep promoted head operations within the current inline budget
    let line_width = usize::from(context.options.line_width);
    let max_head_len = remaining_width.unwrap_or(line_width);

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
    let cap_member_promotion_before_call_tail =
        starts_with_member && has_call_like_tail && remaining_width.is_none();

    // accumulate simple operations while within the promotion limits
    let mut head_len = base_len;
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
            // only promote the pair when both operations are simple
            let Some(next_operation) = next_operation else {
                break;
            };

            // when a chain has more member hops after a member + call pair:
            // keep fluent chains one hop per line
            let has_later_member_hop = operations.get(index + 2..).is_some_and(|tail| {
                tail.iter()
                    .any(|op| matches!(op, ChainExpression::Member { .. }))
            });
            if has_later_member_hop {
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

            if !is_simple_chain_operation(context, operation)
                || (!is_simple_chain_operation(context, next_operation)
                    && !next_call_can_expand
                    && !next_is_promotable_single_argument_call)
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

            // keep call-start chains restricted to call-like operations
            let next_is_call_like = matches!(
                next_operation,
                ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
            );
            let next_is_numeric_index = matches!(
                next_operation,
                ChainExpression::Index { index, .. } if is_numeric_index(context, index)
            );
            if first_is_call_or_numeric_index && !(next_is_call_like || next_is_numeric_index) {
                break;
            }

            // stop if promoting the pair would make the head too long
            let member_len = chain_operation_len(context, operation);
            let next_len = chain_head_operation_len(context, next_operation);
            let combined_len = head_len.saturating_add(member_len).saturating_add(next_len);
            if combined_len > max_head_len
                && !(next_call_can_expand && combined_len <= line_width)
                && !(next_is_promotable_single_argument_call && combined_len <= line_width)
            {
                break;
            }

            head_len = combined_len;
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

        // stop if promoting this operation would make the head too long
        let operation_len = chain_head_operation_len(context, operation);
        let next_len = head_len.saturating_add(operation_len);
        if next_len > max_head_len {
            let allow_direct_curried_tail = matches!(
                operation,
                ChainExpression::Call {
                    position: PostfixPosition::Direct,
                    ..
                }
            ) && previous_op_is_direct_call
                && next_len <= line_width;
            if !allow_direct_curried_tail {
                break;
            }
        }

        head_len = next_len;
        head_ops_count += 1;
        index += 1;
    }

    head_ops_count
}
