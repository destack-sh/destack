use super::groups::{
    MemberChainGroup, TailChainGroups, build_tail_chain_groups, chain_head_member_count,
};
use crate::expression::{
    parenthesized_expression_needs_preserved_wrapper,
    transparent_wrapper_needs_parentheses_in_parent,
};
use crate::operator::{is_chain_expression, write_postfix_base_expression};
use crate::{DestackFormatContext, DestackFormatter};
use destack_core::StringId;
use destack_dir::{
    Argument, Declarator, DecoratorPosition, Expression, GenericArgument, IfForm, LocalNodeId,
    NodeType, PostfixPosition, ScalarLiteral, TokenType, Tree,
};
use destack_fir::format::{Buffer, FormatError, FormatResult};
use destack_fir::prelude::token;
use destack_fir::write;

/// Format a maybe expression without considering chaining.
pub(crate) fn format_maybe_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
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

/// The root printed before the head group members.
#[derive(Clone)]
pub(crate) enum ChainRoot {
    Path {
        node_id: LocalNodeId<Expression>,
        segment: StringId,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        emit_postfix_annotations: bool,
    },
    Expression(LocalNodeId<Expression>),
}

/// One call position inside a normalized chain.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum CallExpressionPosition {
    /// The first call in a call chain.
    Start,
    /// One call between other chain operations.
    Middle,
    /// The outermost call at the end of the chain.
    End,
}

/// One member in an expression chain.
#[derive(Clone)]
pub(crate) enum ChainMember {
    /// Member expression.
    Member {
        node_id: LocalNodeId<Expression>,
        optional_position: Option<PostfixPosition>,
        segment: StringId,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        emit_prefix_annotations: bool,
        emit_postfix_annotations: bool,
    },
    /// Instantiation expression.
    Instantiation {
        node_id: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },
    /// Call expression.
    Call {
        node_id: LocalNodeId<Expression>,
        call_position: CallExpressionPosition,
        optional_position: Option<PostfixPosition>,
        position: PostfixPosition,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Index expression.
    Index {
        node_id: LocalNodeId<Expression>,
        optional_position: Option<PostfixPosition>,
        position: PostfixPosition,
        index: Option<LocalNodeId<Expression>>,
    },
    /// Maybe expression.
    Maybe {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    },
    /// Must expression.
    Must {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    },
}

/// Return the expression node id carried by one chain member.
pub(crate) fn chain_member_node_id(member: &ChainMember) -> LocalNodeId<Expression> {
    match member {
        ChainMember::Member { node_id, .. }
        | ChainMember::Instantiation { node_id, .. }
        | ChainMember::Call { node_id, .. }
        | ChainMember::Index { node_id, .. }
        | ChainMember::Maybe { node_id, .. }
        | ChainMember::Must { node_id, .. } => *node_id,
    }
}

/// Return whether one chain operation is an index access.
pub(crate) fn chain_member_is_index(member: &ChainMember) -> bool {
    matches!(member, ChainMember::Index { .. })
}

/// Return whether one chain operation behaves like a call.
pub(crate) fn chain_member_is_call_like(member: &ChainMember) -> bool {
    matches!(
        member,
        ChainMember::Call { .. } | ChainMember::Instantiation { .. }
    )
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
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => Some(*left),
        _ => None,
    }
}

/// Collect all chain nodes from root to leaf.
pub(crate) fn chain_nodes(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Expression>> {
    let tree = context.tree;
    // walk from leaf to root through chain links
    let mut chain = Vec::new();
    let mut current = node_id;
    loop {
        chain.push(current);

        // semantic transparent wrappers become the base of the remaining chain
        if transparent_wrapper_needs_parentheses_in_parent(context, current) {
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
    context: &DestackFormatContext<'_>,
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
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<(
    Vec<LocalNodeId<Expression>>,
    ChainRoot,
    MemberChainGroup,
    TailChainGroups,
)> {
    let tree = context.tree;
    let chain = chain_nodes(context, node_id);
    let root_id = chain[0];
    let base_root_id = root_id;

    let mut root = ChainRoot::Expression(base_root_id);
    let mut members = Vec::new();

    // path root decomposition
    if let Some((path_root, path_members)) = split_path_chain_root(context, base_root_id)? {
        root = path_root;
        members.extend(path_members);
    }

    let mut chain_tail = chain.iter().skip(1).copied().peekable();
    while let Some(expression_id) = chain_tail.next() {
        if matches!(tree.get(expression_id), Expression::Maybe { .. })
            && chain_tail.peek().is_some_and(|next_id| {
                matches!(
                    tree.get(*next_id),
                    Expression::Member { .. }
                        | Expression::PrivateMember { .. }
                        | Expression::Call { .. }
                        | Expression::Index { .. }
                )
            })
        {
            continue;
        }

        members.push(chain_member_from_node(tree, expression_id)?);
    }

    let head_member_count = chain_head_member_count(context, &root, &members);
    let tail_members = members.split_off(head_member_count);
    annotate_call_chain_positions(context.tree, &mut members, node_id);

    let head = MemberChainGroup::from_members(members);
    let tail_groups = build_tail_chain_groups(context, tail_members);

    Ok((chain, root, head, tail_groups))
}

/// Try to split one path root into a chain base head plus member operations.
fn split_path_chain_root(
    context: &DestackFormatContext<'_>,
    base_root_id: LocalNodeId<Expression>,
) -> FormatResult<Option<(ChainRoot, Vec<ChainMember>)>> {
    let Expression::QualifiedReference {
        path,
        generic_arguments,
    } = context.tree.get(base_root_id)
    else {
        return Ok(None);
    };

    if path.segments.len() <= 1 {
        return Ok(None);
    }

    let segments = &path.segments;
    let generic_arguments = generic_arguments.clone();
    let Some(first_segment) = segments.first().copied() else {
        return Err(FormatError::SyntaxError {
            message: "path chain root must contain at least one segment",
        });
    };

    let tail_segments = &segments[1..];
    let tail_len = tail_segments.len();
    let emit_postfix_on_tail = tail_len > 0
        && path_postfix_annotations_emit_on_tail(context, base_root_id, segments.len());
    let base_generic_arguments = if tail_len == 0 {
        generic_arguments.clone()
    } else {
        Vec::new()
    };
    let root = ChainRoot::Path {
        node_id: base_root_id,
        segment: first_segment,
        generic_arguments: base_generic_arguments,
        emit_postfix_annotations: tail_len == 0 || !emit_postfix_on_tail,
    };

    let mut operations = Vec::with_capacity(tail_len);
    for (index, segment) in tail_segments.iter().copied().enumerate() {
        let is_last = index + 1 == tail_len;
        let generic_arguments = if is_last {
            generic_arguments.clone()
        } else {
            Vec::new()
        };
        operations.push(ChainMember::Member {
            node_id: base_root_id,
            optional_position: None,
            segment,
            generic_arguments,
            emit_prefix_annotations: false,
            emit_postfix_annotations: emit_postfix_on_tail && is_last,
        });
    }

    Ok(Some((root, operations)))
}

/// Check whether source contains a comment between two expression nodes.
pub(crate) fn has_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };

    !context
        .comment_tokens_in_range(between_span.start, between_span.end)
        .is_empty()
}

/// Check whether an optional index is numerically inline.
pub(crate) fn is_numeric_index(
    context: &DestackFormatContext<'_>,
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
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Bigint(_) | ScalarLiteral::Float(_)
        )
    )
}

/// Return an expression end anchor used for trivia checks.
pub(crate) fn expression_trivia_anchor_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression = context.tree.get(expression_id);
    let span = context.span(expression_id);

    // member like nodes often include trailing separator comments in their full spans
    // so anchor at the property token to inspect the comment gap before parent operators
    match expression {
        Expression::Member { .. } | Expression::PrivateMember { .. } => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |member_span| member_span.end),
        Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |path_span| path_span.end),
        Expression::Identifier { .. } => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |path_span| path_span.end),
        _ => context
            .last_non_trivia_token_in_span(span)
            .map_or(span.end, |token| token.span.end),
    }
}

/// Return the property token start for one member-like expression.
pub(crate) fn member_property_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<u32> {
    context.tree.get_main_span(node_id).map(|span| span.start)
}

/// Check if a member access has an intervening comment between receiver and property.
pub(crate) fn member_has_intervening_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(property_start) = member_property_start(context, node_id) else {
        return false;
    };

    let receiver_end = match context.tree.get(node_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            context.span(*left).end
        }
        _ => return false,
    };

    context
        .comments()
        .has_comment_in_range(receiver_end, property_start)
        || context
            .comments()
            .has_end_of_line_comment_after(context.span(node_id).end)
}

/// Check whether a member access uses a private hash (`.#name`).
pub(crate) fn member_is_private_hash(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(context.tree.get(node_id), Expression::PrivateMember { .. }) {
        return true;
    }

    let Some(property_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    let Some(prev_token) = context.token_before_token_start(property_span.start) else {
        return false;
    };

    prev_token.token.ty == TokenType::Hash
}

/// Decide whether postfix annotations on a path belong after the last segment.
pub(crate) fn path_postfix_annotations_emit_on_tail(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> bool {
    let Some(last_segment_start) = path_last_segment_start(context, node_id, segments_len) else {
        return false;
    };

    let mut has_postfix = false;

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        let annotation = context.annotation(annotation_id);
        let position = annotation.position;
        let is_postfix = matches!(
            position,
            DecoratorPosition::LinePostfix
                | DecoratorPosition::BlockInfix
                | DecoratorPosition::BlockPostfix
        );
        if !is_postfix {
            continue;
        }

        has_postfix = true;
        let span = context.annotation_span(annotation_id);
        if span.start < last_segment_start {
            return false;
        }
    }

    if !has_postfix {
        return true;
    }

    true
}

/// Find the start byte of the last path segment token.
fn path_last_segment_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> Option<u32> {
    if segments_len == 0 {
        return None;
    }

    let span = context.span(node_id);
    context.nth_token_type_start_in_span(span, TokenType::Identifier, segments_len)
}

/// Convert one chain expression node into a chain operation.
pub(crate) fn chain_member_from_node(
    tree: &Tree,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<ChainMember> {
    let chain_member = match tree.get(expression_id) {
        Expression::Member { left, name, .. } => {
            let Some(name) = *name else {
                return Err(FormatError::SyntaxError {
                    message: "missing member name in chain expression",
                });
            };
            ChainMember::Member {
                node_id: expression_id,
                optional_position: maybe_position_for_left(tree, *left),
                segment: name,
                generic_arguments: vec![],
                emit_prefix_annotations: false,
                emit_postfix_annotations: true,
            }
        }
        Expression::PrivateMember { left, name, .. } => {
            let Some(name) = *name else {
                return Err(FormatError::SyntaxError {
                    message: "missing private member name in chain expression",
                });
            };
            ChainMember::Member {
                node_id: expression_id,
                optional_position: maybe_position_for_left(tree, *left),
                segment: name,
                generic_arguments: vec![],
                emit_prefix_annotations: false,
                emit_postfix_annotations: true,
            }
        }
        Expression::Call {
            left,
            position,
            generic_arguments,
            arguments,
            ..
        } => ChainMember::Call {
            node_id: expression_id,
            call_position: CallExpressionPosition::End,
            optional_position: maybe_position_for_left(tree, *left),
            position: *position,
            generic_arguments: generic_arguments.clone(),
            arguments: arguments.clone(),
        },
        Expression::Instantiation {
            generic_arguments, ..
        } => ChainMember::Instantiation {
            node_id: expression_id,
            generic_arguments: generic_arguments.clone(),
        },
        Expression::Index {
            left,
            position,
            index,
            ..
        } => ChainMember::Index {
            node_id: expression_id,
            optional_position: maybe_position_for_left(tree, *left),
            position: *position,
            index: *index,
        },
        Expression::Maybe { position, .. } => ChainMember::Maybe {
            node_id: expression_id,
            position: *position,
        },
        Expression::Must { position, .. } => ChainMember::Must {
            node_id: expression_id,
            position: *position,
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
    operations: &mut [ChainMember],
    root_id: LocalNodeId<Expression>,
) {
    for operation in operations {
        let ChainMember::Call {
            node_id,
            call_position,
            ..
        } = operation
        else {
            continue;
        };

        let Expression::Call { left, .. } = tree.get(*node_id) else {
            continue;
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
}

/// Return the optional postfix position stored on one left operand maybe wrapper.
fn maybe_position_for_left(
    tree: &Tree,
    left_id: LocalNodeId<Expression>,
) -> Option<PostfixPosition> {
    let Expression::Maybe { position, .. } = tree.get(left_id) else {
        return None;
    };

    Some(*position)
}

/// Return whether the normalized chain contains at least one call-like operation.
pub(crate) fn chain_has_call_like_expression(
    context: &DestackFormatContext<'_>,
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
    context: &DestackFormatContext<'_>,
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
                    | Expression::AwaitMust { expression }
                    | Expression::Parenthesized { expression } => expression.id == current_id,
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
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;

    loop {
        if context.has_annotation(current_id) {
            return current_id;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } => {
                if parenthesized_expression_needs_preserved_wrapper(
                    context,
                    current_id,
                    *expression,
                ) {
                    None
                } else {
                    Some(*expression)
                }
            }
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
