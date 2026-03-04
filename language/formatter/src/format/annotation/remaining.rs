use destack_ast::{AnnotationPosition, NodeParentIndex, NodeTree, TokenType};
use destack_source::Span;

use super::attachment::following_owner_with_token_after_fallback;
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentSeamContext, CommentSeamData,
};
use super::ownership::{
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_by_shared_start,
};
use super::semicolon::{
    attach_before_comment_semicolon_guard_head, attach_inline_comment_before_semicolon,
    seam_has_line_leading_semicolon_after_comment,
    try_attach_comment_before_empty_statement_semicolon,
};

/// Prepared ctx for one remaining-placement seam comment.
struct RemainingCommentContext<'a, 'ctx> {
    /// The seam ctx.
    ctx: &'a CommentSeamContext<'ctx>,
    /// The seam facts.
    seam: &'a CommentSeamData,
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// Span before the seam.
    token_before_span: Option<Span>,
    /// Owner before the seam from token lookup.
    token_before_owner: Option<u32>,
    /// Span after the seam.
    token_after_span: Option<Span>,
    /// Owner before the seam.
    preceding_owner: Option<u32>,
    /// Owner after the seam.
    following_owner: Option<u32>,
    /// Owner after the seam with token fallback.
    following_owner_with_token_after_fallback: Option<u32>,
    /// Inline block comment classification.
    is_inline_star_comment: bool,
    /// Line-leading semicolon classification for token after comment.
    semicolon_after_is_line_leading: bool,
}

/// Build one remaining-placement seam comment ctx.
fn build_remaining_comment_ctx<'a, 'ctx>(
    ctx: &'a CommentSeamContext<'ctx>,
    seam: &'a CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> RemainingCommentContext<'a, 'ctx> {
    let token_before_span = ctx.token_before_span.map(|token| token.span);
    let token_before_owner =
        token_before_span.and_then(|span| find_smallest_owner_enclosing_token(ctx.tree, span));
    let token_after_span = ctx.token_after_span.map(|token| token.span);
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;
    let following_owner_with_token_after_fallback =
        following_owner_with_token_after_fallback(ctx.tree, ctx, following_owner);
    let is_inline_star_comment =
        seam.comment_is_star && !seam.has_leading_newline && !seam.has_trailing_newline;
    let semicolon_after_is_line_leading = seam_has_line_leading_semicolon_after_comment(ctx, seam);

    RemainingCommentContext {
        ctx,
        seam,
        tree: ctx.tree,
        parents: ctx.parents,
        token_before_span,
        token_before_owner,
        token_after_span,
        preceding_owner,
        following_owner,
        following_owner_with_token_after_fallback,
        is_inline_star_comment,
        semicolon_after_is_line_leading,
    }
}

/// Run one ordered remaining-placement handler sequence.
fn run_remaining_comment_handlers(
    ctx: &RemainingCommentContext<'_, '_>,
    handlers: &[fn(&RemainingCommentContext<'_, '_>) -> Option<CommentAttachment>],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Attach same-line seams before preceding-preferring separators and operators.
fn attach_token_after_prefers_preceding(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_owner: Option<u32>,
    token_before_span: Option<Span>,
    is_inline_star_comment: bool,
) -> Option<CommentAttachment> {
    if !seam.token_after_prefers_preceding {
        return None;
    }

    let token_after_is_close_delimiter_or_separator =
        token_type_is_close_delimiter_or_separator(seam.token_after_type);
    let target_node = if token_after_is_close_delimiter_or_separator {
        token_before_owner.or(preceding_owner)?
    } else {
        preceding_owner.or(token_before_owner)?
    };
    let target_node = if token_after_is_close_delimiter_or_separator {
        normalize_formatter_trivia_target_owner(tree, target_node)
    } else {
        normalize_owner_with_shared_end(tree, parents, target_node, token_before_span)
    };

    // inline block comments before close delimiters and separators should stay boundary-bound
    let position = if is_inline_star_comment && token_after_is_close_delimiter_or_separator {
        AnnotationPosition::LinePostfixBoundary
    } else {
        AnnotationPosition::LinePostfix
    };

    Some((Some(target_node), position))
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

/// Attach empty-statement semicolon ownership for remaining comments.
fn attach_remaining_empty_statement_semicolon_comment(
    ctx: &RemainingCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_before_empty_statement_semicolon(
        ctx.tree,
        ctx.parents,
        ctx.ctx,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner_with_token_after_fallback,
    )
}

/// Attach inline-before-semicolon ownership for remaining comments.
fn attach_remaining_inline_before_semicolon_comment(
    ctx: &RemainingCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_inline_comment_before_semicolon(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.semicolon_after_is_line_leading,
        ctx.preceding_owner,
        ctx.token_before_span,
        ctx.is_inline_star_comment,
    )
}

/// Attach semicolon-guard head ownership for remaining comments.
fn attach_remaining_semicolon_guard_head_comment(
    ctx: &RemainingCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_before_comment_semicolon_guard_head(
        ctx.tree,
        ctx.parents,
        ctx.ctx,
        ctx.seam,
        ctx.following_owner,
    )
}

/// Attach preceding-preferring separator ownership for remaining comments.
fn attach_remaining_preceding_separator_comment(
    ctx: &RemainingCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_token_after_prefers_preceding(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.preceding_owner,
        ctx.token_before_owner,
        ctx.token_before_span,
        ctx.is_inline_star_comment,
    )
}

/// Return whether one token type is a close delimiter or one list separator token.
fn token_type_is_close_delimiter_or_separator(token_type: Option<TokenType>) -> bool {
    matches!(
        token_type,
        Some(
            TokenType::CloseParenthesis
                | TokenType::CloseBracket
                | TokenType::CloseBrace
                | TokenType::Semicolon
        )
    )
}

/// Attach default following-owner ownership for remaining comments.
fn attach_remaining_default_following_comment(
    ctx: &RemainingCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_following_owner(
        ctx.tree,
        ctx.parents,
        ctx.following_owner,
        ctx.token_after_span,
    )
}

/// Attach remaining inline comments when own-line and end-of-line rules do not apply.
pub(crate) fn attach_remaining_comment(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let ctx = build_remaining_comment_ctx(ctx, seam, owners);

    run_remaining_comment_handlers(
        &ctx,
        &[
            attach_remaining_empty_statement_semicolon_comment,
            attach_remaining_inline_before_semicolon_comment,
            attach_remaining_semicolon_guard_head_comment,
            attach_remaining_preceding_separator_comment,
            attach_remaining_default_following_comment,
        ],
    )
}
