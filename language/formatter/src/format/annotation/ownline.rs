use destack_ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenType,
};
use destack_source::Span;

use super::attachment::{
    following_owner_with_token_after_fallback, try_attach_comment_before_empty_statement_semicolon,
};
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner, is_open_delimiter_token,
    previous_non_newline_token_index,
};
use super::ownership::{
    find_smallest_owner_enclosing_token, is_block_like_owner,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_start, promote_owner_to_nearest_statement_boundary,
    promote_owner_to_node_type_ancestor,
};
use super::semicolon::attach_semicolon_guard_own_line_comment;

/// Return whether one owner is an if expression nested in another if expression.
fn owner_is_nested_if_expression(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    if !matches!(
        tree.get(LocalNodeId::<Expression>::new(owner_id)),
        Expression::If { .. }
    ) {
        return false;
    }

    let Some(parent_id) = parents.get_by_id(owner_id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != NodeType::Expression {
        return false;
    }

    matches!(
        tree.get(LocalNodeId::<Expression>::new(parent_id)),
        Expression::If { .. }
    )
}

/// Promote one owner to an expression ancestor when needed.
fn promote_owner_to_expression_ancestor_if_needed(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner: u32,
) -> u32 {
    if tree.get_node_type(owner) == NodeType::Expression {
        return owner;
    }

    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Expression).unwrap_or(owner)
}

/// Resolve one own-line preceding owner with one non-newline token fallback.
fn preceding_owner_with_token_fallback(
    context: &CommentSeamContext<'_>,
    owners: CommentAttachmentNeighbors,
) -> Option<u32> {
    owners.preceding.or_else(|| {
        context
            .token_before
            .and_then(|token_index| {
                previous_non_newline_token_index(context.semantic_tokens, token_index)
            })
            .and_then(|token_index| context.semantic_tokens.get(token_index))
            .and_then(|token| find_smallest_owner_enclosing_token(context.tree, token.span))
    })
}

/// Handle own-line comments before nested else seams.
fn attach_else_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if !seam.token_after_is_keyword(CommentSeamKeyword::Else) {
        return None;
    }

    let enclosing_owner_is_nested_if =
        enclosing_owner.is_some_and(|owner| owner_is_nested_if_expression(tree, parents, owner));
    let following_owner_is_block =
        following_owner.is_some_and(|owner| is_block_like_owner(tree, owner));
    if enclosing_owner_is_nested_if
        && !following_owner_is_block
        && let Some(target_node) = enclosing_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPostfix));
    }

    if let Some(target_node) = following_owner.or(enclosing_owner).or(preceding_owner) {
        let target_node =
            promote_owner_to_expression_ancestor_if_needed(tree, parents, target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if !seam.comment_is_line && !seam.comment_is_multiline_star {
            AnnotationPosition::LinePrefix
        } else {
            AnnotationPosition::BlockPrefix
        };
        return Some((Some(target_node), position));
    }

    None
}

/// Handle own-line comments inside parenthesized groups before `<`.
fn attach_before_less_than_comment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let token_before_is_open_delimiter =
        seam.token_before_type.is_some_and(is_open_delimiter_token);
    if token_before_is_open_delimiter
        && seam.token_after_is(TokenType::LessThan)
        && let Some(target_node) = following_owner.or(enclosing_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}

/// Handle own-line comments after member semicolons.
fn attach_after_member_semicolon_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if seam.token_after_is(TokenType::Semicolon)
        && let Some(target_node) = preceding_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        if let Some(member_owner) =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Member)
        {
            let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
            return Some((Some(member_owner), AnnotationPosition::BlockPostfix));
        }
    }

    None
}

/// Handle own-line line comments before jsx statement heads.
fn attach_before_jsx_statement_head_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    token_after_span: Option<Span>,
) -> Option<CommentAttachment> {
    let token_before_is_statement_end = matches!(
        seam.token_before_type,
        Some(TokenType::Semicolon | TokenType::CloseBrace | TokenType::CloseParenthesis)
    );
    if seam.comment_is_line
        && token_before_is_statement_end
        && seam.token_after_is(TokenType::LessThan)
        && let Some(target_node) = following_owner
    {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}

/// Handle own-line seams before separators and closers that prefer preceding owners.
fn attach_separator_or_closer_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if seam.token_after_prefers_preceding
        && let Some(target_node) = preceding_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);

        let preceding_is_property_or_member =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Member)
                .is_some()
                || promote_owner_to_node_type_ancestor(
                    tree,
                    parents,
                    target_node,
                    NodeType::Property,
                )
                .is_some();

        if seam.comment_is_line && seam.token_after_is(TokenType::CloseBrace) {
            // property/member tail comments should bind following so formatter-owned separators
            // stay before the comment in generated output
            if preceding_is_property_or_member {
                return None;
            }
        }

        let position = if seam.comment_is_line {
            if seam.token_after_is(TokenType::CloseBrace) {
                AnnotationPosition::BlockPostfix
            } else {
                AnnotationPosition::LinePostfixBoundary
            }
        } else {
            AnnotationPosition::BlockPostfix
        };
        return Some((Some(target_node), position));
    }

    None
}

/// Attach own-line line comments before leading statement semicolons to the following statement.
fn attach_before_statement_semicolon_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
    preceding_owner: Option<u32>,
    following_owner_with_token_fallback: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if !seam.comment_is_line || !seam.token_after_is(TokenType::Semicolon) {
        return None;
    }

    if let Some(attachment) =
        attach_semicolon_guard_own_line_comment(tree, parents, context, seam, owners)
    {
        return Some(attachment);
    }

    let target_owner = following_owner_with_token_fallback
        .map(|target_owner| {
            promote_owner_to_nearest_statement_boundary(tree, parents, target_owner)
        })
        .filter(|target_owner| tree.get_node_type(*target_owner) != NodeType::Block)
        .map(|target_owner| normalize_formatter_trivia_target_owner(tree, target_owner));
    if let Some(target_owner) = target_owner {
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    let target_owner = preceding_owner.or_else(|| {
        token_before_span.and_then(|span| find_smallest_owner_enclosing_token(tree, span))
    })?;
    let target_owner =
        normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Attach own-line comments with dedicated own-line rules.
pub(crate) fn attach_own_line_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline {
        return None;
    }

    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let preceding_owner = preceding_owner_with_token_fallback(context, owners);
    let following_owner = owners.following;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache);
    let following_owner_with_token_fallback =
        following_owner_with_token_after_fallback(tree, context, following_owner);

    // empty-statement semicolon ownership
    if let Some(attachment) = try_attach_comment_before_empty_statement_semicolon(
        tree,
        parents,
        seam,
        following_owner_with_token_fallback,
    ) {
        return Some(attachment);
    }

    // statement semicolon comment ownership
    if let Some(attachment) = attach_before_statement_semicolon_comment(
        tree,
        parents,
        context,
        seam,
        owners,
        preceding_owner,
        following_owner_with_token_fallback,
        token_before_span,
    ) {
        return Some(attachment);
    }

    // nested else ownership
    if let Some(attachment) = attach_else_comment(
        tree,
        parents,
        seam,
        preceding_owner,
        following_owner,
        enclosing_owner,
        token_before_span,
    ) {
        return Some(attachment);
    }

    // delimiter interior less-than ownership
    if let Some(attachment) =
        attach_before_less_than_comment(tree, seam, following_owner, enclosing_owner)
    {
        return Some(attachment);
    }

    // member semicolon ownership
    if let Some(attachment) = attach_after_member_semicolon_comment(
        tree,
        parents,
        seam,
        preceding_owner,
        token_before_span,
    ) {
        return Some(attachment);
    }

    // jsx statement head ownership
    if let Some(attachment) = attach_before_jsx_statement_head_comment(
        tree,
        parents,
        seam,
        following_owner,
        token_after_span,
    ) {
        return Some(attachment);
    }

    // separator and closer ownership
    if let Some(attachment) =
        attach_separator_or_closer_comment(tree, parents, seam, preceding_owner, token_before_span)
    {
        return Some(attachment);
    }

    None
}
