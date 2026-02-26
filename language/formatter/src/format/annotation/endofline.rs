use destack_ast::{
    AnnotationPosition, Argument, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenType,
};
use destack_source::Span;

use super::attachment::{
    attach_line_comment_after_ternary_colon, attach_star_comment_before_ternary_colon,
    attach_trailing_comma_close_brace_property_line_comment,
    following_owner_with_token_after_fallback, if_expression_then_owner_without_else,
    try_attach_comment_before_empty_statement_semicolon,
};
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, comment_enclosing_owner, previous_non_newline_token_index,
};
use super::ownership::{
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_by_shared_start,
    promote_owner_to_nearest_statement_boundary, promote_owner_to_node_type_ancestor,
};

/// Resolve one preceding owner with one previous non-newline token fallback owner.
fn preceding_owner_with_previous_non_newline_token_fallback(
    context: &CommentSeamContext<'_>,
    preceding_owner: Option<u32>,
) -> Option<u32> {
    let fallback_owner = context
        .token_before
        .and_then(|token_index| {
            previous_non_newline_token_index(context.semantic_tokens, token_index)
        })
        .and_then(|token_index| context.semantic_tokens.get(token_index))
        .and_then(|token| find_smallest_owner_enclosing_token(context.tree, token.span));

    preceding_owner.or(fallback_owner)
}

/// Attach one same-line line comment after one empty if statement.
fn attach_empty_if_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner_with_semicolon_fallback: Option<u32>,
    enclosing_owner: Option<u32>,
    token_before_span: Option<Span>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_before_is(TokenType::Semicolon) {
        return None;
    }

    let preceding_if_owner = preceding_owner_with_semicolon_fallback
        .map(|owner| normalize_owner_with_shared_end(tree, parents, owner, token_before_span));
    for candidate_owner in [preceding_if_owner, enclosing_owner].into_iter().flatten() {
        if let Some(then_owner) = if_expression_then_owner_without_else(tree, candidate_owner) {
            return Some((Some(then_owner), AnnotationPosition::LinePostfixBoundary));
        }
    }

    None
}

/// Attach one same-line line comment after one trailing comma before `}`.
fn attach_trailing_comma_close_brace_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam.token_before_is(TokenType::Comma)
        || !seam.token_after_is(TokenType::CloseBrace)
    {
        return None;
    }

    attach_trailing_comma_close_brace_property_line_comment(
        tree,
        parents,
        context.token_before_span,
        preceding_owner,
    )
}

/// Attach one same-line comment after one semicolon-terminated statement.
fn attach_after_semicolon_terminated_statement_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner_with_semicolon_fallback: Option<u32>,
    token_before_span: Option<Span>,
    is_same_line_line_comment: bool,
    is_same_line_trailing_block_comment: bool,
) -> Option<CommentAttachment> {
    if !seam.token_before_is(TokenType::Semicolon)
        || (!is_same_line_line_comment && !is_same_line_trailing_block_comment)
    {
        return None;
    }

    let target_node = preceding_owner_with_semicolon_fallback?;
    let target_node =
        normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
    if let Some(member_owner) =
        promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Member)
    {
        let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
        return Some((Some(member_owner), AnnotationPosition::LinePostfixBoundary));
    }

    let target_node = promote_owner_to_nearest_statement_boundary(tree, parents, target_node);
    Some((Some(target_node), AnnotationPosition::LinePostfixBoundary))
}

/// Attach one same-line line comment after one inline break or continue statement.
fn attach_inline_break_or_continue_comment(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment {
        return None;
    }

    let token_before_span = context.token_before_span?;
    let target_node = find_smallest_owner_enclosing_token(tree, token_before_span.span)?;
    if tree.get_node_type(target_node) != NodeType::Expression {
        return None;
    }

    let target_expression = LocalNodeId::<Expression>::new(target_node);
    if !matches!(
        tree.get(target_expression),
        Expression::Break { .. } | Expression::Continue { .. }
    ) {
        return None;
    }

    Some((Some(target_node), AnnotationPosition::LinePostfixBoundary))
}

/// Attach one same-line line comment after one ternary `:` seam.
fn attach_after_ternary_colon_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam.token_before_is(TokenType::Colon)
        || seam.token_before_is_return_type_colon
    {
        return None;
    }

    attach_line_comment_after_ternary_colon(tree, parents, preceding_owner, token_before_span)
}

/// Attach one block comment before one ternary `:` seam.
fn attach_before_ternary_colon_comment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.comment_is_star
        || !seam.token_after_is(TokenType::Colon)
        || seam.token_before_is_return_type_colon
    {
        return None;
    }

    attach_star_comment_before_ternary_colon(tree, preceding_owner, following_owner)
}

/// Attach one same-line line comment before one tree-expression container `}`.
fn attach_before_tree_expression_close_brace_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_after_is(TokenType::CloseBrace) {
        return None;
    }

    let preceding_expression_owner = preceding_owner
        .filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)
        .filter(|owner| {
            parents
                .get_by_id(*owner)
                .is_some_and(|parent_id| tree.get_node_type(parent_id) == NodeType::Argument)
        })?;
    Some((
        Some(preceding_expression_owner),
        AnnotationPosition::LinePostfixBoundary,
    ))
}

/// Attach one same-line line comment before one argument container `}`.
fn attach_before_argument_close_brace_comment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    enclosing_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_after_is(TokenType::CloseBrace) {
        return None;
    }

    let target_owner = preceding_owner
        .filter(|owner| tree.get_node_type(*owner) == NodeType::Argument)
        .or_else(|| {
            enclosing_owner.filter(|owner| tree.get_node_type(*owner) == NodeType::Argument)
        })?;
    let argument_id = LocalNodeId::<Argument>::new(target_owner);
    let value_id = match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };
    Some((Some(value_id.id), AnnotationPosition::LinePostfixBoundary))
}

/// Attach one following-bound seam comment as one line-prefix comment.
fn attach_following_binding_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    token_after_span: Option<Span>,
) -> Option<CommentAttachment> {
    let colon_prefers_preceding = seam.token_before_is(TokenType::Colon)
        && !seam.token_before_is_return_type_colon
        && seam.comment_is_line;
    if !seam.seam_binds_right || seam.token_after_prefers_preceding || colon_prefers_preceding {
        return None;
    }

    let target_owner = following_owner?;
    let target_owner = token_after_span
        .map(|span| promote_owner_by_shared_start(tree, parents, target_owner, span.start))
        .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Attach one comment after one member semicolon seam to the member owner.
fn attach_member_semicolon_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if !seam.token_before_is(TokenType::Semicolon) {
        return None;
    }

    let target_node = preceding_owner?;
    let target_node =
        normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
    let member_owner =
        promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Member)?;
    let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
    Some((Some(member_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Attach end-of-line comments and terminal seam comments.
pub(crate) fn attach_end_of_line_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.has_trailing_newline && context.token_after.is_some() {
        return None;
    }

    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;
    let following_owner_with_token_fallback =
        following_owner_with_token_after_fallback(tree, context, following_owner);
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let preceding_owner_with_semicolon_fallback =
        preceding_owner_with_previous_non_newline_token_fallback(context, preceding_owner);
    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache);
    let is_same_line_line_comment = seam.comment_is_line && !seam.has_leading_newline;
    let is_same_line_trailing_block_comment =
        seam.comment_is_star && !seam.has_leading_newline && seam.has_trailing_newline;

    // empty-statement semicolon ownership
    if let Some(attachment) = try_attach_comment_before_empty_statement_semicolon(
        tree,
        parents,
        seam,
        following_owner_with_token_fallback,
    ) {
        return Some(attachment);
    }

    // empty-if same-line comment ownership
    if let Some(attachment) = attach_empty_if_comment(
        tree,
        parents,
        seam,
        preceding_owner_with_semicolon_fallback,
        enclosing_owner,
        token_before_span,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // trailing-comma close-brace ownership
    if let Some(attachment) = attach_trailing_comma_close_brace_comment(
        tree,
        parents,
        context,
        seam,
        preceding_owner,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // semicolon-terminated statement ownership
    if let Some(attachment) = attach_after_semicolon_terminated_statement_comment(
        tree,
        parents,
        seam,
        preceding_owner_with_semicolon_fallback,
        token_before_span,
        is_same_line_line_comment,
        is_same_line_trailing_block_comment,
    ) {
        return Some(attachment);
    }

    // inline break or continue ownership
    if let Some(attachment) =
        attach_inline_break_or_continue_comment(tree, context, is_same_line_line_comment)
    {
        return Some(attachment);
    }

    // ternary colon trailing ownership
    if let Some(attachment) = attach_after_ternary_colon_comment(
        tree,
        parents,
        seam,
        preceding_owner,
        token_before_span,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // ternary colon prefix star ownership
    if let Some(attachment) =
        attach_before_ternary_colon_comment(tree, seam, preceding_owner, following_owner)
    {
        return Some(attachment);
    }

    // tree-expression close-brace ownership
    if let Some(attachment) = attach_before_tree_expression_close_brace_comment(
        tree,
        parents,
        seam,
        preceding_owner,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // argument close-brace ownership
    if let Some(attachment) = attach_before_argument_close_brace_comment(
        tree,
        seam,
        preceding_owner,
        enclosing_owner,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // following-binding ownership
    if let Some(attachment) =
        attach_following_binding_comment(tree, parents, seam, following_owner, token_after_span)
    {
        return Some(attachment);
    }

    // member semicolon ownership
    if let Some(attachment) =
        attach_member_semicolon_comment(tree, parents, seam, preceding_owner, token_before_span)
    {
        return Some(attachment);
    }

    None
}
