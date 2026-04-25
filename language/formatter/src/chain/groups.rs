use std::cell::Cell;
use std::collections::VecDeque;

use super::{
    ChainMember, ChainRoot, chain_member_is_call_like, chain_member_node_id,
    expression_trivia_anchor_end, is_numeric_index, transparent_inner_expression,
};
use crate::DestackFormatContext;
use destack_ast::{DecoratorPosition, Expression, PostfixPosition};
use smallvec::SmallVec;

/// One member-chain group.
#[derive(Clone)]
pub(crate) struct MemberChainGroup {
    members: SmallVec<[ChainMember; 2]>,
    will_break: Cell<bool>,
    needs_empty_line: Cell<bool>,
}

impl MemberChainGroup {
    /// Build one group from a member list.
    pub(crate) fn from_members(members: Vec<ChainMember>) -> Self {
        Self {
            members: SmallVec::from_vec(members),
            will_break: Cell::new(false),
            needs_empty_line: Cell::new(false),
        }
    }

    /// Build one group from its first operation.
    fn new(member: ChainMember) -> Self {
        Self {
            members: SmallVec::from_iter([member]),
            will_break: Cell::new(false),
            needs_empty_line: Cell::new(false),
        }
    }

    /// Push one member into the group.
    fn push(&mut self, member: ChainMember) {
        self.members.push(member);
    }

    /// Return the first member in the group.
    pub(crate) fn first(&self) -> Option<&ChainMember> {
        self.members.first()
    }

    /// Return the last member in the group.
    pub(crate) fn last(&self) -> Option<&ChainMember> {
        self.members.last()
    }

    /// Return the members as one slice.
    pub(crate) fn members(&self) -> &[ChainMember] {
        self.members.as_slice()
    }

    /// Return an iterator over all members in the group.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &ChainMember> {
        self.members.iter()
    }

    /// Extend the group with more members.
    pub(crate) fn extend_members(&mut self, members: impl IntoIterator<Item = ChainMember>) {
        self.members.extend(members);
    }

    /// Consume the group and return its members.
    pub(crate) fn into_members(self) -> SmallVec<[ChainMember; 2]> {
        self.members
    }

    /// Record whether the formatted group breaks.
    pub(crate) fn set_will_break(&self, will_break: bool) {
        self.will_break.set(will_break);
    }

    /// Return whether the formatted group breaks.
    pub(crate) fn will_break(&self) -> bool {
        self.will_break.get()
    }

    /// Record whether an empty line precedes the group.
    pub(crate) fn set_needs_empty_line(&self, needs_empty_line: bool) {
        self.needs_empty_line.set(needs_empty_line);
    }

    /// Return whether an empty line precedes the group.
    pub(crate) fn needs_empty_line(&self) -> bool {
        self.needs_empty_line.get()
    }
}

/// Build tail groups after the chain head.
#[derive(Default)]
struct TailChainGroupsBuilder {
    groups: VecDeque<MemberChainGroup>,
    current_group: Option<MemberChainGroup>,
}

impl TailChainGroupsBuilder {
    /// Start a new tail group.
    fn start_group(&mut self, member: ChainMember) {
        debug_assert!(self.current_group.is_none());
        self.current_group = Some(MemberChainGroup::new(member));
    }

    /// Append to the current group or start a new one.
    fn start_or_continue_group(&mut self, member: ChainMember) {
        match &mut self.current_group {
            None => self.start_group(member),
            Some(group) => group.push(member),
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
    groups: VecDeque<MemberChainGroup>,
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
    pub(crate) fn first(&self) -> Option<&MemberChainGroup> {
        self.groups.front()
    }

    /// Return the last tail group.
    pub(crate) fn last(&self) -> Option<&MemberChainGroup> {
        self.groups.back()
    }

    /// Remove and return the first tail group.
    pub(crate) fn pop_first(&mut self) -> Option<MemberChainGroup> {
        self.groups.pop_front()
    }

    /// Return an iterator over all tail groups.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &MemberChainGroup> {
        self.groups.iter()
    }

    /// Return one tail group by index.
    pub(crate) fn get(&self, index: usize) -> Option<&MemberChainGroup> {
        self.groups.get(index)
    }

    /// Return one mutable tail group by index.
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut MemberChainGroup> {
        self.groups.get_mut(index)
    }

    /// Return whether any group except the last breaks.
    pub(crate) fn any_except_last_will_break(&self) -> bool {
        let count = self.groups.len().saturating_sub(1);
        self.groups
            .iter()
            .take(count)
            .any(MemberChainGroup::will_break)
    }

    /// Return whether this chain has multiple tail groups.
    pub(crate) fn is_member_call_chain(&self) -> bool {
        self.groups.len() > 1
    }
}

/// Return the number of members that stay in the head group.
pub(crate) fn chain_head_member_count(
    context: &DestackFormatContext<'_>,
    root: &ChainRoot,
    members: &[ChainMember],
) -> usize {
    if members.is_empty() {
        return 0;
    }

    if members
        .first()
        .is_some_and(|member| chain_member_has_leading_gap_comment(context, member))
    {
        return 0;
    }

    if chain_root_has_leading_call_like(context, root, members) {
        return members
            .iter()
            .position(|member| !chain_member_stays_in_leading_head(context, member))
            .unwrap_or(members.len());
    }

    let non_call_or_numeric_index_start = members
        .iter()
        .position(|member| !chain_member_stays_in_leading_head(context, member))
        .unwrap_or(members.len());

    let rest = &members[non_call_or_numeric_index_start..];
    let member_end = rest
        .iter()
        .position(|member| !chain_member_extends_member_head(member))
        .map_or(rest.len(), |index| index.saturating_sub(1));

    non_call_or_numeric_index_start + member_end
}

/// Return whether one chain member owns a source comment before its leading token.
pub(crate) fn chain_member_has_leading_gap_comment(
    context: &DestackFormatContext<'_>,
    member: &ChainMember,
) -> bool {
    let node_id = chain_member_node_id(member);
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
    tail_members: Vec<ChainMember>,
) -> TailChainGroups {
    let mut groups_builder = TailChainGroupsBuilder::default();
    let mut has_seen_call_like = false;
    let mut members = tail_members.into_iter().peekable();

    while let Some(member) = members.next() {
        let next_is_instantiation = members
            .peek()
            .is_some_and(|member| matches!(member, ChainMember::Instantiation { .. }));

        if chain_member_is_numeric_direct_index(context, &member) {
            groups_builder.start_or_continue_group(member);
        } else if chain_member_is_member_like(&member) {
            if has_seen_call_like && !next_is_instantiation {
                groups_builder.close_group();
                groups_builder.start_group(member);
                has_seen_call_like = false;
            } else {
                groups_builder.start_or_continue_group(member);
                if next_is_instantiation {
                    has_seen_call_like = false;
                }
            }
        } else if chain_member_is_call_or_attached_tail(&member) {
            let is_call_like = matches!(member, ChainMember::Call { .. });
            groups_builder.start_or_continue_group(member);
            if is_call_like {
                has_seen_call_like = true;
            }
        } else {
            groups_builder.close_group();
            groups_builder.start_group(member);
            has_seen_call_like = false;
        }

        let current_group = &groups_builder.current_group;
        if current_group
            .as_ref()
            .and_then(MemberChainGroup::last)
            .is_some_and(|member| {
                chain_member_has_trailing_annotations(context, member)
                    || chain_member_has_trailing_comment(context, member)
            })
        {
            groups_builder.close_group();
            has_seen_call_like = false;
        }
    }

    groups_builder.finish()
}

/// Return whether one chain member has one source-adjacent trailing comment.
fn chain_member_has_trailing_comment(
    context: &DestackFormatContext<'_>,
    member: &ChainMember,
) -> bool {
    let member_span = context.span(chain_member_node_id(member));

    context
        .comments()
        .comments_after(member_span.end)
        .first()
        .is_some_and(|comment| {
            context
                .source_text()
                .all_bytes_match(member_span.end, comment.span.start, |byte| {
                    byte.is_ascii_whitespace()
                })
        })
}

/// Return whether one base begins with call-like chaining.
fn chain_root_has_leading_call_like(
    context: &DestackFormatContext<'_>,
    root: &ChainRoot,
    head_members: &[ChainMember],
) -> bool {
    match root {
        ChainRoot::Expression(expression_id) => {
            let expression_id = transparent_inner_expression(context, *expression_id);
            let expression = context.tree.get(expression_id);
            chain_expression_is_call_like_base(context, expression)
                || head_members.first().is_some_and(chain_member_is_call_like)
        }
        ChainRoot::Path { .. } => head_members.first().is_some_and(chain_member_is_call_like),
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

/// Return whether one member stays in the leading head run.
fn chain_member_stays_in_leading_head(
    context: &DestackFormatContext<'_>,
    member: &ChainMember,
) -> bool {
    chain_member_is_call_like(member)
        || chain_member_is_numeric_direct_index(context, member)
        || matches!(member, ChainMember::Maybe { .. } | ChainMember::Must { .. })
}

/// Return whether one member extends the member head.
fn chain_member_extends_member_head(member: &ChainMember) -> bool {
    chain_member_is_member_like(member)
}

/// Return whether one member is a numeric direct index.
fn chain_member_is_numeric_direct_index(
    context: &DestackFormatContext<'_>,
    member: &ChainMember,
) -> bool {
    matches!(
        member,
        ChainMember::Index {
            position: PostfixPosition::Direct,
            index,
            ..
        } if is_numeric_index(context, index)
    )
}

/// Return whether one member is member-like.
fn chain_member_is_member_like(member: &ChainMember) -> bool {
    matches!(
        member,
        ChainMember::Member { .. } | ChainMember::Index { .. }
    )
}

/// Return whether one member should stay attached to the current group.
fn chain_member_is_call_or_attached_tail(member: &ChainMember) -> bool {
    chain_member_is_call_like(member)
        || matches!(member, ChainMember::Maybe { .. } | ChainMember::Must { .. })
}

/// Return whether one member has postfix annotations that force a split.
fn chain_member_has_trailing_annotations(
    context: &DestackFormatContext<'_>,
    member: &ChainMember,
) -> bool {
    context
        .annotation_ids(chain_member_node_id(member))
        .iter()
        .any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id).position,
                DecoratorPosition::LinePostfix | DecoratorPosition::BlockPostfix
            )
        })
}
