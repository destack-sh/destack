use destack_ast::{AnnotationPosition, NodeParentIndex, NodeTree, TokenType};
use destack_source::Span;

use super::attachment::{
    following_owner_with_token_after_fallback, try_attach_comment_before_empty_statement_semicolon,
};
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentSeamContext, CommentSeamData,
};
use super::ownership::{
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_by_shared_start,
};
use super::semicolon::{SemicolonGuardCommentSeam, classify_semicolon_guard_comment_seam};

/// Attach remaining comments before empty-statement semicolons.
fn attach_before_empty_statement_semicolon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner_with_token_fallback: Option<u32>,
) -> Option<CommentAttachment> {
    try_attach_comment_before_empty_statement_semicolon(
        tree,
        parents,
        seam,
        following_owner_with_token_fallback,
    )
}

/// Attach same-line inline block comments before semicolons.
fn attach_inline_block_before_semicolon(
    tree: &NodeTree,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if !seam.comment_is_star
        || seam.has_leading_newline
        || seam.has_trailing_newline
        || !seam.token_after_is(TokenType::Semicolon)
    {
        return None;
    }

    let target_node = preceding_owner.or_else(|| {
        token_before_span.and_then(|span| find_smallest_owner_enclosing_token(tree, span))
    })?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::LinePostfixBoundary))
}

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

/// Attach same-line semicolon guard seams to the guarded following expression.
fn attach_after_semicolon_guard_head(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    token_after_span: Option<Span>,
) -> Option<CommentAttachment> {
    let semicolon_guard_seam = classify_semicolon_guard_comment_seam(
        context.semantic_tokens,
        seam.token_before_type,
        seam.token_after_type,
        context.token_after,
    );
    if !matches!(
        semicolon_guard_seam,
        SemicolonGuardCommentSeam::BeforeComment { .. }
    ) {
        return None;
    }

    let target_node = following_owner?;
    let target_node = token_after_span
        .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
        .unwrap_or(target_node);
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::LinePrefix))
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
    if let Some(attachment) = attach_before_empty_statement_semicolon(
        tree,
        parents,
        seam,
        following_owner_with_token_fallback,
    ) {
        return Some(attachment);
    }

    // inline block comment before semicolon
    if let Some(attachment) =
        attach_inline_block_before_semicolon(tree, seam, preceding_owner, token_before_span)
    {
        return Some(attachment);
    }

    // semicolon guard head ownership
    if let Some(attachment) = attach_after_semicolon_guard_head(
        tree,
        parents,
        context,
        seam,
        following_owner,
        token_after_span,
    ) {
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
