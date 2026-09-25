use super::groups::{
    MemberChainGroup, TailChainGroups, build_tail_chain_groups, chain_head_member_count,
};
use crate::expression::should_preserve_source_parentheses;
use crate::operator::{is_chain_expression, write_postfix_base_expression};
use crate::{TsppFormatContext, TsppFormatter};
use tspp_dir::{
    Declarator, Expression, IfForm, Literal, LocalNodeId, NodeType, PostfixPosition, Tree,
};
use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::token;
use tspp_fir::write;

/// Format a maybe expression without considering chaining.
pub(crate) fn format_maybe_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Maybe { left, position } = f.context().tree.get(node_id) {
        write_postfix_base_expression(f, *left)?;
        match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => write!(f, [token("."), token("?")])?,
        }
    } else {
        return Err(FormatError::SyntaxError {
            message: "unexpected expression kind for maybe formatter",
        });
    }
    Ok(())
}

/// One call position inside a normalized chain.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum CallExpressionPosition {
    /// The first call in a call chain.
    Start,
    /// One call between other chain operations.
    Middle,
    /// The outermost call at the end of the chain.
    End,
}

/// One member in an expression chain.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ChainMember {
    /// Member expression.
    Member { node_id: LocalNodeId<Expression> },
    /// Instantiation expression.
    Instantiation { node_id: LocalNodeId<Expression> },
    /// Call expression.
    Call {
        /// The source expression.
        node_id: LocalNodeId<Expression>,
        /// The position among adjacent calls.
        call_position: CallExpressionPosition,
    },
    /// Index expression.
    Index { node_id: LocalNodeId<Expression> },
    /// Maybe expression.
    Maybe { node_id: LocalNodeId<Expression> },
    /// Must expression.
    Must { node_id: LocalNodeId<Expression> },
}

impl ChainMember {
    /// Return the expression node id carried by this member.
    pub(crate) const fn node_id(&self) -> LocalNodeId<Expression> {
        match self {
            Self::Member { node_id, .. }
            | Self::Instantiation { node_id, .. }
            | Self::Call { node_id, .. }
            | Self::Index { node_id, .. }
            | Self::Maybe { node_id, .. }
            | Self::Must { node_id, .. } => *node_id,
        }
    }

    /// Return whether this member is an index access.
    pub(crate) const fn is_index(&self) -> bool {
        matches!(self, Self::Index { .. })
    }

    /// Return whether this member behaves like a call.
    pub(crate) const fn is_call_like(&self) -> bool {
        matches!(self, Self::Call { .. } | Self::Instantiation { .. })
    }

    /// Return the source expression after validating the member kind.
    pub(crate) fn expression<'tree>(&self, tree: &'tree Tree) -> FormatResult<&'tree Expression> {
        let expression = tree.get(self.node_id());
        let has_matching_kind = matches!(
            (self, expression),
            (Self::Member { .. }, Expression::Member { .. })
                | (Self::Instantiation { .. }, Expression::Instantiation { .. })
                | (Self::Call { .. }, Expression::Call { .. })
                | (Self::Index { .. }, Expression::Index { .. })
                | (Self::Maybe { .. }, Expression::Maybe { .. })
                | (Self::Must { .. }, Expression::Must { .. })
        );
        if !has_matching_kind {
            return Err(FormatError::SyntaxError {
                message: "chain member does not match its source expression",
            });
        }

        Ok(expression)
    }
}

/// Return the first member of the first tail group.
pub(crate) fn first_tail_group_member(tail_groups: &TailChainGroups) -> Option<&ChainMember> {
    tail_groups.first().and_then(|group| group.first())
}

/// Return the left operand for one chain node.
pub(crate) fn chain_node_left_id(
    tree: &Tree,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(node_id) {
        Expression::Member { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::Chain { expression: left } => Some(*left),
        _ => None,
    }
}

/// Collect all chain nodes from root to leaf.
pub(crate) fn chain_nodes(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Expression>> {
    let tree = context.tree;
    // walk from leaf to root through chain links
    let mut chain = Vec::new();
    let mut current = node_id;
    loop {
        chain.push(current);

        // semantic transparent wrappers become the base of the remaining chain
        if should_preserve_source_parentheses(context, current) {
            break;
        }

        let Some(next_id) = chain_node_left_id(tree, current) else {
            break;
        };
        current = next_id;
    }
    chain.reverse();

    chain
}

/// Return whether one expression has a ternary expression ancestor.
pub(crate) fn expression_has_ternary_ancestor(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;

    while let Some((parent_id, parent_type)) = context.parent(current_id) {
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        if matches!(
            context.tree.get(parent_expression_id),
            Expression::If {
                form: IfForm::Ternary,
                ..
            }
        ) {
            return true;
        }

        current_id = parent_expression_id;
    }

    false
}

/// Build the normalized chain, base, and tail groups for one chain expression.
pub(super) fn build_member_chain_parts(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<(
    Vec<LocalNodeId<Expression>>,
    LocalNodeId<Expression>,
    MemberChainGroup,
    TailChainGroups,
)> {
    let tree = context.tree;
    let chain = chain_nodes(context, node_id);
    let root_id = chain[0];
    let mut members = Vec::new();

    let mut chain_tail = chain.iter().skip(1).copied().peekable();
    while let Some(expression_id) = chain_tail.next() {
        if matches!(tree.get(expression_id), Expression::Maybe { .. })
            && chain_tail.peek().is_some_and(|next_id| {
                matches!(
                    tree.get(*next_id),
                    Expression::Member { .. } | Expression::Call { .. } | Expression::Index { .. }
                )
            })
        {
            continue;
        }

        members.push(chain_member_from_node(tree, expression_id)?);
    }

    let head_member_count = chain_head_member_count(context, root_id, &members)?;
    let tail_members = members.split_off(head_member_count);
    annotate_call_chain_positions(context.tree, &mut members, node_id)?;

    let head = MemberChainGroup::from_members(members);
    let tail_groups = build_tail_chain_groups(context, tail_members)?;

    Ok((chain, root_id, head, tail_groups))
}

/// Check whether source contains a comment between two expression nodes.
pub(crate) fn has_comment_between_expressions(
    context: &TsppFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };

    !context
        .source_comments_in_range(between_span.start, between_span.end)
        .is_empty()
}

/// Check whether an optional index is numerically inline.
pub(crate) fn is_numeric_index(
    context: &TsppFormatContext<'_>,
    index: &Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(index_id) = *index else {
        return false;
    };
    if context.has_annotation(index_id) {
        return false;
    }

    matches!(
        context.tree.get(index_id),
        Expression::Literal(Literal::Integer(_) | Literal::Bigint(_) | Literal::Float(_))
    )
}

/// Return an expression end anchor used for trivia checks.
pub(crate) fn expression_trivia_anchor_end(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression = context.tree.get(expression_id);
    let span = context.span(expression_id);

    // member like nodes often include trailing separator comments in their full spans
    // so anchor at the property token to inspect the comment gap before parent operators
    match expression {
        Expression::Member { .. } => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |member_span| member_span.end),
        Expression::Identifier { .. } => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |path_span| path_span.end),
        _ => context
            .last_token_in_span(span)
            .map_or(span.end, |token| token.span.end),
    }
}

/// Return the property token start for one member-like expression.
pub(crate) fn member_property_start(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<u32> {
    context.tree.get_main_span(node_id).map(|span| span.start)
}

/// Check if a member access has an intervening comment between receiver and property.
pub(crate) fn member_has_intervening_comment(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(property_start) = member_property_start(context, node_id) else {
        return false;
    };

    let receiver_end = match context.tree.get(node_id) {
        Expression::Member { left, .. } => context.span(*left).end,
        _ => return false,
    };

    context
        .comments()
        .has_comment_in_range(receiver_end, property_start)
        || context
            .comments()
            .has_end_of_line_comment_after(context.span(node_id).end)
}

/// Convert one chain expression node into a chain operation.
pub(crate) fn chain_member_from_node(
    tree: &Tree,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<ChainMember> {
    let chain_member = match tree.get(expression_id) {
        Expression::Member { name, .. } => {
            if name.is_none() {
                return Err(FormatError::SyntaxError {
                    message: "missing member name in chain expression",
                });
            }

            ChainMember::Member {
                node_id: expression_id,
            }
        }
        Expression::Call { .. } => ChainMember::Call {
            node_id: expression_id,
            call_position: CallExpressionPosition::End,
        },
        Expression::Instantiation { .. } => ChainMember::Instantiation {
            node_id: expression_id,
        },
        Expression::Index { .. } => ChainMember::Index {
            node_id: expression_id,
        },
        Expression::Maybe { .. } => ChainMember::Maybe {
            node_id: expression_id,
        },
        Expression::Must { .. } => ChainMember::Must {
            node_id: expression_id,
        },
        _ => {
            return Err(FormatError::SyntaxError {
                message: "unexpected expression kind for chain expression",
            });
        }
    };

    Ok(chain_member)
}

/// Annotate every call operation with its position inside the chain.
fn annotate_call_chain_positions(
    tree: &Tree,
    members: &mut [ChainMember],
    root_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    for member in members {
        let ChainMember::Call {
            node_id,
            call_position,
            ..
        } = member
        else {
            continue;
        };

        let Expression::Call { left, .. } = tree.get(*node_id) else {
            return Err(FormatError::SyntaxError {
                message: "call chain member does not match its source expression",
            });
        };

        let left_is_chain = is_chain_expression(tree.get(*left));
        *call_position = if !left_is_chain {
            CallExpressionPosition::Start
        } else if *node_id == root_id {
            CallExpressionPosition::End
        } else {
            CallExpressionPosition::Middle
        };
    }

    Ok(())
}

/// Return the optional postfix position stored on one left operand maybe wrapper.
pub(crate) fn access_marker_position(
    tree: &Tree,
    left_id: LocalNodeId<Expression>,
    is_optional: bool,
) -> Option<PostfixPosition> {
    // optional accesses print `?.`; absorbed maybe receivers keep their authored marker
    if is_optional {
        return Some(PostfixPosition::Direct);
    }
    let Expression::Maybe { position, .. } = tree.get(left_id) else {
        return None;
    };

    Some(*position)
}

/// Return whether the normalized chain contains at least one call-like operation.
pub(crate) fn chain_has_call_like_expression(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    chain_nodes(context, node_id)
        .into_iter()
        .any(|expression_id| {
            matches!(
                tree.get(expression_id),
                Expression::Call { .. } | Expression::Instantiation { .. }
            )
        })
}

/// Walk upward through transparent wrappers to find an assignment-like parent rhs.
pub(crate) fn assignment_like_parent(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<(NodeType, u32)> {
    let mut current_id = node_id.id;

    // walk through transparent wrappers until we reach an assignment-like parent
    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        match parent_type {
            NodeType::Expression => {
                let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));

                // assignment rhs
                if let Expression::Assign { right, .. } = parent_expr
                    && right.id == current_id
                {
                    return Some((NodeType::Expression, parent_id));
                }

                // transparent wrappers around the rhs
                let is_wrapper_parent = match parent_expr {
                    Expression::Await { expression }
                    | Expression::AwaitMaybe { expression }
                    | Expression::AwaitMust { expression } => expression.id == current_id,
                    _ => false,
                };

                if is_wrapper_parent {
                    current_id = parent_id;
                    continue;
                }

                return None;
            }
            NodeType::Declarator => {
                let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
                if declarator.value.is_some_and(|value| value.id == current_id) {
                    return Some((NodeType::Declarator, parent_id));
                }
                return None;
            }
            _ => return None,
        }
    }

    None
}

/// Unwrap transparent wrappers around an expression for classification.
pub(crate) fn transparent_inner_expression(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;

    loop {
        if context.has_annotation(current_id) {
            return current_id;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Await { expression }
            | Expression::AwaitMaybe { expression }
            | Expression::AwaitMust { expression } => Some(*expression),
            _ => None,
        };

        let Some(next_id) = next_id else {
            return current_id;
        };

        current_id = next_id;
    }
}
