use std::collections::VecDeque;

use super::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead, chain_operation_is_call_like,
    chain_operation_node_id, expression_trivia_anchor_end, is_numeric_index,
    transparent_inner_expression,
};
use crate::DestackFormatContext;
use destack_ast::{DecoratorPosition, Expression, PostfixPosition};
use smallvec::SmallVec;

/// One tail group following the chain head.
#[derive(Clone)]
pub(crate) struct TailChainGroup {
    operations: SmallVec<[ChainExpression; 2]>,
    will_break: bool,
    needs_empty_line: bool,
}

impl TailChainGroup {
    /// Build one group from its first operation.
    fn new(operation: ChainExpression) -> Self {
        Self {
            operations: SmallVec::from_iter([operation]),
            will_break: false,
            needs_empty_line: false,
        }
    }

    /// Push one operation into the group.
    fn push(&mut self, operation: ChainExpression) {
        self.operations.push(operation);
    }

    /// Return the first operation in the group.
    pub(crate) fn first(&self) -> Option<&ChainExpression> {
        self.operations.first()
    }

    /// Return the last operation in the group.
    pub(crate) fn last(&self) -> Option<&ChainExpression> {
        self.operations.last()
    }

    /// Return the operations as one slice.
    pub(crate) fn as_slice(&self) -> &[ChainExpression] {
        self.operations.as_slice()
    }

    /// Return an iterator over all operations in the group.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &ChainExpression> {
        self.operations.iter()
    }

    /// Record whether the group breaks when formatted.
    pub(crate) fn set_will_break(&mut self, will_break: bool) {
        self.will_break = will_break;
    }

    /// Return whether the formatted group breaks.
    pub(crate) fn will_break(&self) -> bool {
        self.will_break
    }

    /// Record whether an empty line precedes the group.
    pub(crate) fn set_needs_empty_line(&mut self, needs_empty_line: bool) {
        self.needs_empty_line = needs_empty_line;
    }

    /// Return whether an empty line precedes the group.
    pub(crate) fn needs_empty_line(&self) -> bool {
        self.needs_empty_line
    }
}

/// Build tail groups after the chain head.
#[derive(Default)]
struct TailChainGroupsBuilder {
    groups: VecDeque<TailChainGroup>,
    current_group: Option<TailChainGroup>,
}

impl TailChainGroupsBuilder {
    /// Start a new tail group.
    fn start_group(&mut self, operation: ChainExpression) {
        debug_assert!(self.current_group.is_none());
        self.current_group = Some(TailChainGroup::new(operation));
    }

    /// Append to the current group or start a new one.
    fn start_or_continue_group(&mut self, operation: ChainExpression) {
        match &mut self.current_group {
            None => self.start_group(operation),
            Some(group) => group.push(operation),
        }
    }

    /// Close the current group.
    fn close_group(&mut self) {
        if let Some(group) = self.current_group.take() {
            self.groups.push_back(group);
        }
    }

    /// Finish building all tail groups.
    fn finish(self) -> TailChainGroups {
        let mut groups = self.groups;

        if let Some(group) = self.current_group {
            groups.push_back(group);
        }

        TailChainGroups { groups }
    }
}

/// The groups following the chain head.
#[derive(Clone)]
pub(crate) struct TailChainGroups {
    groups: VecDeque<TailChainGroup>,
}

impl TailChainGroups {
    /// Return whether there are no tail groups.
    pub(crate) fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Return the number of tail groups.
    pub(crate) fn len(&self) -> usize {
        self.groups.len()
    }

    /// Return the first tail group.
    pub(crate) fn first(&self) -> Option<&TailChainGroup> {
        self.groups.front()
    }

    /// Return the last tail group.
    pub(crate) fn last(&self) -> Option<&TailChainGroup> {
        self.groups.back()
    }

    /// Remove and return the first tail group.
    pub(crate) fn pop_first(&mut self) -> Option<TailChainGroup> {
        self.groups.pop_front()
    }

    /// Return an iterator over all tail groups.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &TailChainGroup> {
        self.groups.iter()
    }

    /// Return one tail group by index.
    pub(crate) fn get(&self, index: usize) -> Option<&TailChainGroup> {
        self.groups.get(index)
    }

    /// Return one mutable tail group by index.
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut TailChainGroup> {
        self.groups.get_mut(index)
    }

    /// Return whether any group except the last breaks.
    pub(crate) fn any_except_last_will_break(&self) -> bool {
        let count = self.groups.len().saturating_sub(1);
        self.groups
            .iter()
            .take(count)
            .any(TailChainGroup::will_break)
    }
}

/// Return the number of operations that stay in the head group.
pub(crate) fn chain_head_operation_count(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
    operations: &[ChainExpression],
) -> usize {
    if operations
        .first()
        .is_some_and(|operation| chain_operation_has_leading_gap_comment(context, operation))
    {
        return 0;
    }

    if chain_base_has_leading_call_like(context, base) {
        return operations
            .iter()
            .position(|operation| !chain_operation_stays_in_leading_head(context, operation))
            .unwrap_or(operations.len());
    }

    let non_call_or_numeric_index_start = operations
        .iter()
        .position(|operation| !chain_operation_stays_in_leading_head(context, operation))
        .unwrap_or(operations.len());

    let rest = &operations[non_call_or_numeric_index_start..];
    let member_end = rest
        .iter()
        .position(|operation| !chain_operation_extends_member_head(operation))
        .map_or(rest.len(), |index| index.saturating_sub(1));

    non_call_or_numeric_index_start + member_end
}

/// Return whether one chain operation owns a source comment before its leading token.
pub(crate) fn chain_operation_has_leading_gap_comment(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    let node_id = chain_operation_node_id(operation);
    let Some(left_id) = super::member::chain_node_left_id(context.tree, node_id) else {
        return false;
    };

    let left_end = expression_trivia_anchor_end(context, left_id);
    let operation_end = context.span(node_id).end;
    let Some(operation_start) = context
        .first_non_trivia_token_between(left_end, operation_end)
        .map(|token| token.span.start)
    else {
        return false;
    };

    context
        .comments()
        .has_comment_in_range(left_end, operation_start)
}

/// Build the tail groups after the head.
pub(crate) fn build_tail_chain_groups(
    context: &DestackFormatContext<'_>,
    tail_operations: Vec<ChainExpression>,
) -> TailChainGroups {
    let mut groups_builder = TailChainGroupsBuilder::default();
    let mut has_seen_call_like = false;
    let mut operations = tail_operations.into_iter().peekable();

    while let Some(operation) = operations.next() {
        let next_is_instantiation = operations
            .peek()
            .is_some_and(|operation| matches!(operation, ChainExpression::Instantiation { .. }));

        if chain_operation_is_numeric_direct_index(context, &operation) {
            groups_builder.start_or_continue_group(operation);
        } else if chain_operation_is_member_like(&operation) {
            if has_seen_call_like && !next_is_instantiation {
                groups_builder.close_group();
                groups_builder.start_group(operation);
                has_seen_call_like = false;
            } else {
                groups_builder.start_or_continue_group(operation);
                if next_is_instantiation {
                    has_seen_call_like = false;
                }
            }
        } else if chain_operation_is_call_or_attached_tail(&operation) {
            let is_call_like = matches!(operation, ChainExpression::Call { .. });
            groups_builder.start_or_continue_group(operation);
            if is_call_like {
                has_seen_call_like = true;
            }
        } else {
            groups_builder.close_group();
            groups_builder.start_group(operation);
            has_seen_call_like = false;
        }

        let current_group = &groups_builder.current_group;
        if current_group
            .as_ref()
            .and_then(TailChainGroup::last)
            .is_some_and(|operation| {
                chain_operation_has_trailing_annotations(context, operation)
                    || chain_operation_has_trailing_comment(context, operation)
            })
        {
            groups_builder.close_group();
            has_seen_call_like = false;
        }
    }

    groups_builder.finish()
}

/// Return whether one chain operation has one source-adjacent trailing comment.
fn chain_operation_has_trailing_comment(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    let operation_span = context.span(chain_operation_node_id(operation));

    context
        .comments()
        .comments_after(operation_span.end)
        .first()
        .is_some_and(|comment| {
            context
                .source_text()
                .all_bytes_match(operation_span.end, comment.span.start, |byte| {
                    byte.is_ascii_whitespace()
                })
        })
}

/// Return whether one base begins with call-like chaining.
fn chain_base_has_leading_call_like(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
) -> bool {
    match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => {
            let expression_id = transparent_inner_expression(context, *expression_id);
            let expression = context.tree.get(expression_id);
            chain_expression_is_call_like_base(context, expression)
                || base.body.first().is_some_and(chain_operation_is_call_like)
        }
        ChainExpressionBaseHead::Path { .. } => {
            base.body.first().is_some_and(chain_operation_is_call_like)
        }
    }
}

/// Return whether one expression behaves like a call-like chain base.
fn chain_expression_is_call_like_base(
    context: &DestackFormatContext<'_>,
    expression: &Expression,
) -> bool {
    let _ = context;

    matches!(
        expression,
        Expression::Call { .. } | Expression::Instantiation { .. }
    )
}

/// Return whether one operation stays in the leading head run.
fn chain_operation_stays_in_leading_head(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    chain_operation_is_call_like(operation)
        || chain_operation_is_numeric_direct_index(context, operation)
        || matches!(
            operation,
            ChainExpression::Maybe { .. } | ChainExpression::Must { .. }
        )
}

/// Return whether one operation extends the member head.
fn chain_operation_extends_member_head(operation: &ChainExpression) -> bool {
    chain_operation_is_member_like(operation)
        || matches!(
            operation,
            ChainExpression::Maybe { .. } | ChainExpression::Must { .. }
        )
}

/// Return whether one operation is a numeric direct index.
fn chain_operation_is_numeric_direct_index(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    matches!(
        operation,
        ChainExpression::Index {
            position: PostfixPosition::Direct,
            index,
            ..
        } if is_numeric_index(context, index)
    )
}

/// Return whether one operation is member-like.
fn chain_operation_is_member_like(operation: &ChainExpression) -> bool {
    matches!(
        operation,
        ChainExpression::Member { .. } | ChainExpression::Index { .. }
    )
}

/// Return whether one operation should stay attached to the current group.
fn chain_operation_is_call_or_attached_tail(operation: &ChainExpression) -> bool {
    chain_operation_is_call_like(operation)
        || matches!(
            operation,
            ChainExpression::Maybe { .. } | ChainExpression::Must { .. }
        )
}

/// Return whether an operation has postfix annotations that force a split.
fn chain_operation_has_trailing_annotations(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    context
        .annotation_ids(chain_operation_node_id(operation))
        .iter()
        .any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id).position,
                DecoratorPosition::LinePostfix | DecoratorPosition::BlockPostfix
            )
        })
}
