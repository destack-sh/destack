use super::{
    ChainMember, expression_trivia_anchor_end, is_numeric_index, transparent_inner_expression,
};
use crate::TsppFormatContext;
use smallvec::SmallVec;
use tspp_dir::{DecoratorPosition, Expression, LocalNodeId, PostfixPosition};
use tspp_fir::format::{FormatError, FormatResult};

/// One member-chain group.
pub(crate) struct MemberChainGroup {
    members: SmallVec<[ChainMember; 2]>,
}

impl MemberChainGroup {
    /// Build one group from a member list.
    pub(crate) fn from_members(members: Vec<ChainMember>) -> Self {
        Self {
            members: SmallVec::from_vec(members),
        }
    }

    /// Build one group from its first operation.
    fn new(member: ChainMember) -> Self {
        Self {
            members: SmallVec::from_iter([member]),
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
}

/// Build tail groups after the chain head.
#[derive(Default)]
struct TailChainGroupsBuilder {
    groups: Vec<MemberChainGroup>,
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
            self.groups.push(group);
        }
    }

    /// Finish building all tail groups.
    fn finish(self) -> TailChainGroups {
        let mut groups = self.groups;

        if let Some(group) = self.current_group {
            groups.push(group);
        }

        TailChainGroups { groups }
    }
}

/// The groups following the chain head.
pub(crate) struct TailChainGroups {
    groups: Vec<MemberChainGroup>,
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
        self.groups.first()
    }

    /// Return the last tail group.
    pub(crate) fn last(&self) -> Option<&MemberChainGroup> {
        self.groups.last()
    }

    /// Remove and return the first tail group.
    pub(crate) fn pop_first(&mut self) -> Option<MemberChainGroup> {
        if self.groups.is_empty() {
            None
        } else {
            Some(self.groups.remove(0))
        }
    }

    /// Return an iterator over all tail groups.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &MemberChainGroup> {
        self.groups.iter()
    }

    /// Return whether this chain has multiple tail groups.
    pub(crate) fn is_member_call_chain(&self) -> bool {
        self.groups.len() > 1
    }
}

/// Return the number of members that stay in the head group.
pub(crate) fn chain_head_member_count(
    context: &TsppFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
    members: &[ChainMember],
) -> FormatResult<usize> {
    if members.is_empty() {
        return Ok(0);
    }

    if members
        .first()
        .is_some_and(|member| chain_member_has_leading_gap_comment(context, member))
    {
        return Ok(0);
    }

    if chain_root_has_leading_call_like(context, root_id, members) {
        for (index, member) in members.iter().enumerate() {
            if !chain_member_stays_in_leading_head(context, member)? {
                return Ok(index);
            }
        }

        return Ok(members.len());
    }

    let mut non_call_or_numeric_index_start = members.len();
    for (index, member) in members.iter().enumerate() {
        if !chain_member_stays_in_leading_head(context, member)? {
            non_call_or_numeric_index_start = index;
            break;
        }
    }

    let rest = &members[non_call_or_numeric_index_start..];
    let member_end = rest
        .iter()
        .position(|member| !chain_member_extends_member_head(member))
        .map_or(rest.len(), |index| index.saturating_sub(1));

    Ok(non_call_or_numeric_index_start + member_end)
}

/// Return whether one chain member owns a source comment before its leading token.
pub(crate) fn chain_member_has_leading_gap_comment(
    context: &TsppFormatContext<'_>,
    member: &ChainMember,
) -> bool {
    let node_id = member.node_id();
    let Some(left_id) = super::member::chain_node_left_id(context.tree, node_id) else {
        return false;
    };

    let left_end = expression_trivia_anchor_end(context, left_id);
    let operation_end = context.span(node_id).end;
    let Some(operation_start) = context
        .first_token_between(left_end, operation_end)
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
    context: &TsppFormatContext<'_>,
    tail_members: Vec<ChainMember>,
) -> FormatResult<TailChainGroups> {
    let mut groups_builder = TailChainGroupsBuilder::default();
    let mut has_seen_call_like = false;
    let mut members = tail_members.into_iter().peekable();

    while let Some(member) = members.next() {
        let next_is_instantiation = members
            .peek()
            .is_some_and(|member| matches!(member, ChainMember::Instantiation { .. }));

        if chain_member_is_numeric_direct_index(context, &member)? {
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

    Ok(groups_builder.finish())
}

/// Return whether one chain member has one source-adjacent trailing comment.
fn chain_member_has_trailing_comment(
    context: &TsppFormatContext<'_>,
    member: &ChainMember,
) -> bool {
    let member_span = context.span(member.node_id());

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
    context: &TsppFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
    head_members: &[ChainMember],
) -> bool {
    let root_id = transparent_inner_expression(context, root_id);
    let expression = context.tree.get(root_id);

    chain_expression_is_call_like_base(expression)
        || head_members.first().is_some_and(ChainMember::is_call_like)
}

/// Return whether one expression behaves like a call-like chain base.
fn chain_expression_is_call_like_base(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Call { .. } | Expression::Instantiation { .. }
    )
}

/// Return whether one member stays in the leading head run.
fn chain_member_stays_in_leading_head(
    context: &TsppFormatContext<'_>,
    member: &ChainMember,
) -> FormatResult<bool> {
    let stays = member.is_call_like()
        || chain_member_is_numeric_direct_index(context, member)?
        || matches!(member, ChainMember::Maybe { .. } | ChainMember::Must { .. });

    Ok(stays)
}

/// Return whether one member extends the member head.
fn chain_member_extends_member_head(member: &ChainMember) -> bool {
    chain_member_is_member_like(member)
}

/// Return whether one member is a numeric direct index.
fn chain_member_is_numeric_direct_index(
    context: &TsppFormatContext<'_>,
    member: &ChainMember,
) -> FormatResult<bool> {
    let ChainMember::Index { .. } = member else {
        return Ok(false);
    };
    let expression = member.expression(context.tree)?;
    let Expression::Index {
        position, index, ..
    } = expression
    else {
        return Err(FormatError::SyntaxError {
            message: "index chain member does not contain an index expression",
        });
    };
    Ok(*position == PostfixPosition::Direct && is_numeric_index(context, index))
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
    member.is_call_like() || matches!(member, ChainMember::Maybe { .. } | ChainMember::Must { .. })
}

/// Return whether one member has postfix annotations that force a split.
fn chain_member_has_trailing_annotations(
    context: &TsppFormatContext<'_>,
    member: &ChainMember,
) -> bool {
    context
        .annotation_ids(member.node_id())
        .iter()
        .any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id).position,
                DecoratorPosition::LinePostfix | DecoratorPosition::BlockPostfix
            )
        })
}
