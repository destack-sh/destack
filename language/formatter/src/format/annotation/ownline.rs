use destack_ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenType,
};
use destack_source::Span;

use super::attachment::{
    following_owner_with_token_after_fallback,
    preceding_owner_with_non_newline_token_before_fallback,
    preceding_owner_with_token_before_fallback, promote_owner_to_tree_expression_parent,
};
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner, is_open_delimiter_token,
    previous_non_newline_token_index,
};
use super::expression::{
    first_dynamic_argument_owner_for_call_like, promote_owner_to_call_like_expression_ancestor,
};
use super::facts::next_non_newline_token_type_after_seam;
use super::ownership::{
    find_preferred_owner_starting_at, find_smallest_owner_enclosing_token, is_block_like_owner,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_start, promote_owner_to_node_type_ancestor,
};
use super::semicolon::{
    attach_own_line_comment_before_member_semicolon,
    attach_own_line_comment_before_statement_semicolon,
    try_attach_comment_before_empty_statement_semicolon,
};

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
    parents: &NodeParentIndex,
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
        let target_node = promote_owner_to_tree_expression_parent(tree, parents, target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
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
    if (seam.comment_is_line || seam.comment_is_star)
        && token_before_is_statement_end
        && seam.token_after_is(TokenType::LessThan)
        && let Some(target_node) = following_owner
    {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = promote_owner_to_tree_expression_parent(tree, parents, target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}

/// Handle own-line comments between call callee and first argument.
fn attach_call_argument_head_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    token_before_span: Option<Span>,
    seam: &CommentSeamData,
) -> Option<CommentAttachment> {
    if !seam.token_before_is(TokenType::OpenParenthesis) {
        return None;
    }

    let token_before_span = token_before_span?;
    let token_before_owner = find_smallest_owner_enclosing_token(tree, token_before_span)?;
    let token_before_owner = normalize_formatter_trivia_target_owner(tree, token_before_owner);
    let call_like_owner =
        promote_owner_to_call_like_expression_ancestor(tree, parents, token_before_owner)?;
    let target_owner = first_dynamic_argument_owner_for_call_like(tree, call_like_owner)?;
    let target_owner_span = tree.get_span_by_id(target_owner);
    if token_before_span.start >= target_owner_span.start {
        return None;
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);

    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Handle own-line import or export specifier separator comments as dependency-item prefixes.
fn attach_dependency_item_separator_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let seam_follows_comma = seam.token_before_is(TokenType::Comma)
        || context
            .token_before
            .and_then(|token_before| {
                previous_non_newline_token_index(context.semantic_tokens, token_before)
            })
            .and_then(|token_index| context.semantic_tokens.get(token_index))
            .is_some_and(|token| token.token.ty == TokenType::Comma);
    if !seam_follows_comma {
        return None;
    }

    let token_after_owner = context
        .token_after_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let target_owner = [following_owner, token_after_owner]
        .into_iter()
        .flatten()
        .find_map(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::DependencyItem)
        })?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);

    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Handle own-line dependency-item comments after trailing commas before `}`.
fn attach_dependency_item_trailing_comma_close_brace_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.token_before_is(TokenType::Comma) || !seam.token_after_is(TokenType::CloseBrace) {
        return None;
    }

    let token_before_owner = context
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let target_owner = [preceding_owner, token_before_owner]
        .into_iter()
        .flatten()
        .find_map(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::DependencyItem)
        })?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);

    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle own-line seams before separators and closers that prefer preceding owners.
fn attach_separator_or_closer_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if seam.token_after_is(TokenType::Dot) {
        return None;
    }

    let token_after_is_elementwise_operator = matches!(
        seam.token_after_type,
        Some(TokenType::ElementwiseAnd | TokenType::ElementwiseOr | TokenType::ElementwiseXor)
    );
    if token_after_is_elementwise_operator {
        return None;
    }

    if seam.token_after_prefers_preceding {
        let target_node = preceding_owner?;
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        let preceding_is_dependency_item = promote_owner_to_node_type_ancestor(
            tree,
            parents,
            target_node,
            NodeType::DependencyItem,
        )
        .is_some();

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
                if preceding_is_dependency_item {
                    AnnotationPosition::LinePostfixBoundary
                } else {
                    AnnotationPosition::BlockPostfix
                }
            } else {
                AnnotationPosition::LinePostfixBoundary
            }
        } else if seam.token_after_is(TokenType::CloseBrace) && preceding_is_dependency_item {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::BlockPostfix
        };
        return Some((Some(target_node), position));
    }

    None
}

/// Attach own-line comments before member dots to the following member segment.
fn attach_before_member_dot_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    let token_after_is_dot = next_non_newline_token_type_after_seam(
        context.semantic_tokens,
        seam.token_after_type,
        context.token_after,
    ) == Some(TokenType::Dot);
    if !seam.comment_is_line || !token_after_is_dot {
        return None;
    }

    if seam.token_before_is(TokenType::Comma) || seam.token_before_is(TokenType::OpenBrace) {
        return None;
    }

    let target_owner = token_before_span
        .and_then(|span| find_smallest_owner_enclosing_token(tree, span))
        .or_else(|| token_before_span.and_then(|span| find_preferred_owner_starting_at(tree, span)))
        .or(preceding_owner)?;
    let target_owner = promote_owner_to_expression_ancestor_if_needed(tree, parents, target_owner);
    let target_owner =
        normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);

    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Prepared context for one own-line seam comment.
struct OwnLineCommentContext<'a, 'ctx> {
    /// The seam context.
    context: &'a CommentSeamContext<'ctx>,
    /// The seam facts.
    seam: &'a CommentSeamData,
    /// Neighbor owners.
    owners: CommentAttachmentNeighbors,
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// Owner before seam with token fallback.
    preceding_owner: Option<u32>,
    /// Owner after seam.
    following_owner: Option<u32>,
    /// Span before seam.
    token_before_span: Option<Span>,
    /// Span after seam.
    token_after_span: Option<Span>,
    /// Enclosing owner.
    enclosing_owner: Option<u32>,
    /// Owner after seam with token fallback.
    following_owner_with_token_after_fallback: Option<u32>,
}

/// Build one own-line seam comment context.
fn build_own_line_comment_context<'a, 'ctx>(
    context: &'a CommentSeamContext<'ctx>,
    seam: &'a CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> OwnLineCommentContext<'a, 'ctx> {
    let tree = context.tree;
    let parents = context.parents;
    let preceding_owner =
        preceding_owner_with_token_before_fallback(tree, context, owners.preceding);
    let preceding_owner =
        preceding_owner_with_non_newline_token_before_fallback(tree, context, preceding_owner);
    let following_owner = owners.following;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache);
    let following_owner_with_token_after_fallback =
        following_owner_with_token_after_fallback(tree, context, following_owner);

    OwnLineCommentContext {
        context,
        seam,
        owners,
        tree,
        parents,
        preceding_owner,
        following_owner,
        token_before_span,
        token_after_span,
        enclosing_owner,
        following_owner_with_token_after_fallback,
    }
}

/// Run one ordered own-line handler sequence.
fn run_own_line_comment_handlers(
    comment_context: &OwnLineCommentContext<'_, '_>,
    handlers: &[fn(&OwnLineCommentContext<'_, '_>) -> Option<CommentAttachment>],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(comment_context) {
            return Some(attachment);
        }
    }

    None
}

/// Attach empty-statement semicolon ownership for own-line comments.
fn attach_own_line_empty_statement_semicolon_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_before_empty_statement_semicolon(
        comment_context.tree,
        comment_context.parents,
        comment_context.context,
        comment_context.seam,
        comment_context.preceding_owner,
        comment_context.following_owner_with_token_after_fallback,
    )
}

/// Attach statement-semicolon ownership for own-line comments.
fn attach_own_line_statement_semicolon_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_own_line_comment_before_statement_semicolon(
        comment_context.tree,
        comment_context.parents,
        comment_context.context,
        comment_context.seam,
        comment_context.owners,
        comment_context.preceding_owner,
        comment_context.following_owner_with_token_after_fallback,
        comment_context.token_before_span,
    )
}

/// Attach nested-else ownership for own-line comments.
fn attach_own_line_nested_else_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_else_comment(
        comment_context.tree,
        comment_context.parents,
        comment_context.seam,
        comment_context.preceding_owner,
        comment_context.following_owner,
        comment_context.enclosing_owner,
        comment_context.token_before_span,
    )
}

/// Attach delimiter interior `<` ownership for own-line comments.
fn attach_own_line_less_than_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_before_less_than_comment(
        comment_context.tree,
        comment_context.parents,
        comment_context.seam,
        comment_context.following_owner,
        comment_context.enclosing_owner,
    )
}

/// Attach member-semicolon ownership for own-line comments.
fn attach_own_line_member_semicolon_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_own_line_comment_before_member_semicolon(
        comment_context.tree,
        comment_context.parents,
        comment_context.seam,
        comment_context.preceding_owner,
        comment_context.token_before_span,
    )
}

/// Attach jsx statement-head ownership for own-line comments.
fn attach_own_line_jsx_statement_head_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_before_jsx_statement_head_comment(
        comment_context.tree,
        comment_context.parents,
        comment_context.seam,
        comment_context.following_owner,
        comment_context.token_after_span,
    )
}

/// Attach call-argument head ownership for own-line comments.
fn attach_own_line_call_argument_head_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_call_argument_head_comment(
        comment_context.tree,
        comment_context.parents,
        comment_context.token_before_span,
        comment_context.seam,
    )
}

/// Attach member-dot prefix ownership for own-line comments.
fn attach_own_line_member_dot_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_before_member_dot_comment(
        comment_context.tree,
        comment_context.parents,
        comment_context.context,
        comment_context.seam,
        comment_context.preceding_owner,
        comment_context.token_before_span,
    )
}

/// Attach dependency-item separator ownership for own-line comments.
fn attach_own_line_dependency_item_separator_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_dependency_item_separator_comment(
        comment_context.tree,
        comment_context.parents,
        comment_context.context,
        comment_context.seam,
        comment_context.following_owner,
    )
}

/// Attach dependency-item trailing-comma ownership for own-line comments.
fn attach_own_line_dependency_item_trailing_comma_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_dependency_item_trailing_comma_close_brace_comment(
        comment_context.tree,
        comment_context.parents,
        comment_context.context,
        comment_context.seam,
        comment_context.preceding_owner,
    )
}

/// Attach separator and closer ownership for own-line comments.
fn attach_own_line_separator_or_closer_comment(
    comment_context: &OwnLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_separator_or_closer_comment(
        comment_context.tree,
        comment_context.parents,
        comment_context.seam,
        comment_context.preceding_owner,
        comment_context.token_before_span,
    )
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

    let comment_context =
        build_own_line_comment_context(context, seam, enclosing_owner_cache, owners);

    run_own_line_comment_handlers(
        &comment_context,
        &[
            attach_own_line_empty_statement_semicolon_comment,
            attach_own_line_statement_semicolon_comment,
            attach_own_line_nested_else_comment,
            attach_own_line_less_than_comment,
            attach_own_line_member_semicolon_comment,
            attach_own_line_jsx_statement_head_comment,
            attach_own_line_call_argument_head_comment,
            attach_own_line_member_dot_comment,
            attach_own_line_dependency_item_separator_comment,
            attach_own_line_dependency_item_trailing_comma_comment,
            attach_own_line_separator_or_closer_comment,
        ],
    )
}
