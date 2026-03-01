use destack_ast::{AnnotationPosition, NodeParentIndex, NodeTree};
use destack_source::Span;

use super::attachment::following_owner_with_token_after_fallback;
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentSeamContext, CommentSeamData,
};
use super::ownership::{
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_start,
};
use super::semicolon::{
    attach_before_comment_semicolon_guard_head, attach_inline_comment_before_semicolon,
    try_attach_comment_before_empty_statement_semicolon,
};

/// Attach same-line seams before preceding-preferring separators and operators.
fn attach_token_after_prefers_preceding(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if !seam.token_after_prefers_preceding {
        return None;
    }

    let target_node = preceding_owner?;
    let target_node =
        normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
    Some((Some(target_node), AnnotationPosition::LinePostfix))
}

/// Attach same-line seams to following owners as line-prefix comments.
fn attach_following_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    following_owner: Option<u32>,
    token_after_span: Option<Span>,
) -> Option<CommentAttachment> {
    let target_node = following_owner?;
    let target_node = token_after_span
        .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
        .unwrap_or(target_node);
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Attach remaining inline comments when own-line and end-of-line rules do not apply.
pub(crate) fn attach_remaining_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;
    let following_owner_with_token_fallback =
        following_owner_with_token_after_fallback(tree, context, following_owner);

    // empty-statement semicolon ownership
    if let Some(attachment) = try_attach_comment_before_empty_statement_semicolon(
        tree,
        parents,
        context,
        seam,
        preceding_owner,
        following_owner_with_token_fallback,
    ) {
        return Some(attachment);
    }

    // same-line comment before semicolon ownership
    let is_inline_star_comment =
        seam.comment_is_star && !seam.has_leading_newline && !seam.has_trailing_newline;
    if let Some(attachment) = attach_inline_comment_before_semicolon(
        tree,
        parents,
        seam,
        preceding_owner,
        token_before_span,
        is_inline_star_comment,
    ) {
        return Some(attachment);
    }

    // semicolon guard head ownership
    if let Some(attachment) =
        attach_before_comment_semicolon_guard_head(tree, parents, context, seam, following_owner)
    {
        return Some(attachment);
    }

    // preceding-preferring separators and operators
    if let Some(attachment) = attach_token_after_prefers_preceding(
        tree,
        parents,
        seam,
        preceding_owner,
        token_before_span,
    ) {
        return Some(attachment);
    }

    // default following-owner ownership
    if let Some(attachment) =
        attach_following_owner(tree, parents, following_owner, token_after_span)
    {
        return Some(attachment);
    }

    None
}
