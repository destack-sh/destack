use destack_ast::{
    AnnotationPosition, Argument, Expression, IfKind, LocalNodeId, NodeParentIndex, NodeTree,
    NodeType, TokenType, UnaryOperator,
};
use destack_source::Span;

use super::attachment::{
    attach_line_comment_after_ternary_colon, attach_star_comment_before_ternary_colon,
    attach_trailing_comma_close_brace_property_line_comment,
    following_owner_with_token_after_fallback, if_expression_then_owner_without_else,
    preceding_owner_with_non_newline_token_before_fallback,
    promote_owner_to_tree_expression_parent,
};
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner,
    seam_is_template_interpolation_open_brace,
};
use super::expression::{
    first_dynamic_argument_owner_for_call_like, promote_owner_to_call_like_expression_ancestor,
};
use super::ownership::{
    find_preferred_owner_starting_at, find_smallest_owner_enclosing_token,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_start, promote_owner_to_node_type_ancestor,
};
use super::semicolon::{
    attach_after_semicolon_terminated_statement_comment, attach_inline_comment_before_semicolon,
    seam_has_line_leading_semicolon_after_comment, seam_has_line_leading_semicolon_before_comment,
    try_attach_comment_before_empty_statement_semicolon,
};

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

/// Attach one same-line comment after one line-leading semicolon to the following owner.
fn attach_after_line_leading_semicolon_comment(
    tree: &NodeTree,
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner_with_token_after_fallback: Option<u32>,
    is_same_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_comment || !seam_has_line_leading_semicolon_before_comment(ctx, seam) {
        return None;
    }

    let target_owner = following_owner_with_token_after_fallback?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Attach one same-line line comment after one trailing comma before `}`.
fn attach_trailing_comma_close_brace_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ctx: &CommentSeamContext<'_>,
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
        ctx.token_before_span,
        preceding_owner,
    )
}

/// Attach one same-line import or export specifier separator comment to the following dependency item.
fn attach_after_dependency_item_separator_comma_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_before_is(TokenType::Comma) {
        return None;
    }

    let token_before_owner = ctx
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

/// Attach one same-line dependency-item comment before a separator comma.
fn attach_before_dependency_item_separator_comma_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    is_same_line_line_comment: bool,
    is_same_line_trailing_block_comment: bool,
) -> Option<CommentAttachment> {
    if !seam.token_after_is(TokenType::Comma)
        || (!is_same_line_line_comment && !is_same_line_trailing_block_comment)
    {
        return None;
    }

    let token_before_owner = ctx
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let target_owner = [preceding_owner, following_owner, token_before_owner]
        .into_iter()
        .flatten()
        .find_map(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::DependencyItem)
        })?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);

    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Return whether one token type is one additive binary operator token.
#[inline]
fn token_type_is_additive_binary_operator(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::Add
            | TokenType::WrappingAdd
            | TokenType::SaturatingAdd
            | TokenType::Subtract
            | TokenType::WrappingSubtract
            | TokenType::SaturatingSubtract
    )
}

/// Return whether one enclosing owner is one unary additive expression.
fn enclosing_owner_is_unary_additive_expression(
    tree: &NodeTree,
    enclosing_owner: Option<u32>,
) -> bool {
    let Some(enclosing_owner) = enclosing_owner else {
        return false;
    };
    if tree.get_node_type(enclosing_owner) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(enclosing_owner);
    matches!(
        tree.get(expression_id),
        Expression::Unary {
            operator: UnaryOperator::Plus | UnaryOperator::Negate | UnaryOperator::WrappingNegate,
            ..
        }
    )
}

/// Attach one line comment after one additive operator seam.
fn attach_after_binary_operator_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    enclosing_owner: Option<u32>,
    following_owner: Option<u32>,
    token_after_span: Option<Span>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam
            .token_before_type
            .is_some_and(token_type_is_additive_binary_operator)
    {
        return None;
    }

    let target_owner = following_owner.or_else(|| {
        token_after_span.and_then(|token_after_span| {
            find_smallest_owner_enclosing_token(tree, token_after_span)
        })
    })?;
    let target_owner = token_after_span
        .map(|token_after_span| {
            promote_owner_by_shared_start(tree, parents, target_owner, token_after_span.start)
        })
        .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    if enclosing_owner_is_unary_additive_expression(tree, enclosing_owner) {
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Attach one same-line line comment after one inline break or continue statement.
fn attach_inline_break_or_continue_comment(
    tree: &NodeTree,
    ctx: &CommentSeamContext<'_>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment {
        return None;
    }

    let token_before_span = ctx.token_before_span?;
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

/// Attach one same-line interpolation-head line comment to the interpolation owner.
fn attach_template_interpolation_open_brace_line_comment(
    tree: &NodeTree,
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner_with_token_after_fallback: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam_is_template_interpolation_open_brace(ctx, seam) {
        return None;
    }

    let target_node = following_owner_with_token_after_fallback?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockPrefix))
}

/// Attach one same-line line comment after one ternary `:` seam.
fn attach_after_ternary_colon_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
    token_before_span: Option<Span>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam.token_before_is(TokenType::Colon)
        || seam.token_before_is_return_type_colon
    {
        return None;
    }

    let is_switch_case_colon = preceding_owner
        .and_then(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::MatchCase)
        })
        .is_some();
    if is_switch_case_colon {
        return None;
    }

    let seam_has_ternary_expression_context = [enclosing_owner, preceding_owner]
        .into_iter()
        .flatten()
        .any(|owner_id| {
            let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
                Some(owner_id)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
            };

            expression_owner.is_some_and(|expression_owner| {
                let expression_id = LocalNodeId::<Expression>::new(expression_owner);
                matches!(
                    tree.get(expression_id),
                    Expression::If {
                        kind: IfKind::Ternary,
                        ..
                    } | Expression::TypeConditional { .. }
                )
            })
        });
    if !seam_has_ternary_expression_context {
        return None;
    }

    attach_line_comment_after_ternary_colon(
        tree,
        parents,
        preceding_owner,
        following_owner,
        token_before_span,
    )
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
        Argument::Error => return None,
    };
    Some((Some(value_id.id), AnnotationPosition::LinePostfixBoundary))
}

/// Attach one same-line line comment after `(` before one tree-expression head.
fn attach_parenthesized_tree_head_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner_with_token_after_fallback: Option<u32>,
    token_after_span: Option<Span>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam.token_before_is(TokenType::OpenParenthesis)
        || !seam.token_after_is(TokenType::LessThan)
    {
        return None;
    }

    let target_owner = following_owner_with_token_after_fallback?;
    let target_owner = token_after_span
        .map(|span| promote_owner_by_shared_start(tree, parents, target_owner, span.start))
        .unwrap_or(target_owner);
    let target_owner = promote_owner_to_tree_expression_parent(tree, parents, target_owner);
    let target_owner =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Argument)
            .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Attach one call argument head line comment to the first call argument.
fn attach_call_argument_head_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    token_before_span: Option<Span>,
    seam: &CommentSeamData,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_before_is(TokenType::OpenParenthesis) {
        return None;
    }

    let token_before_owner =
        token_before_span.and_then(|span| find_smallest_owner_enclosing_token(tree, span))?;
    let token_before_owner = normalize_formatter_trivia_target_owner(tree, token_before_owner);
    let call_like_owner =
        promote_owner_to_call_like_expression_ancestor(tree, parents, token_before_owner)?;
    let target_owner = first_dynamic_argument_owner_for_call_like(tree, call_like_owner)?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Attach one same-line line comment between one call callee and `(` to the call boundary.
fn attach_call_callee_line_comment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    enclosing_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam.token_after_is(TokenType::OpenParenthesis)
        || seam.token_before_is(TokenType::OpenParenthesis)
    {
        return None;
    }

    let target_owner = enclosing_owner?;
    if tree.get_node_type(target_owner) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(target_owner);
    let supports_callee_comment = match tree.get(expression_id) {
        Expression::Call {
            left,
            static_arguments,
            dynamic_arguments,
            ..
        } => {
            dynamic_arguments.is_empty()
                || static_arguments.is_some()
                || matches!(tree.get(*left), Expression::Instantiation { .. })
        }
        Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
            ..
        } => {
            dynamic_arguments.is_empty()
                || static_arguments.is_some()
                || matches!(tree.get(*left), Expression::Instantiation { .. })
        }
        _ => false,
    };
    if !supports_callee_comment {
        return None;
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Attach one same-line line comment after `as` in import or export items to the dependency item.
fn attach_dependency_item_alias_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_before_is_keyword(CommentSeamKeyword::As) {
        return None;
    }

    let token_before_owner = ctx.token_before_span.and_then(|token| {
        find_preferred_owner_starting_at(tree, token.span)
            .or_else(|| find_smallest_owner_enclosing_token(tree, token.span))
    });
    let token_after_owner = ctx.token_after_span.and_then(|token| {
        find_preferred_owner_starting_at(tree, token.span)
            .or_else(|| find_smallest_owner_enclosing_token(tree, token.span))
    });
    let target_owner = [
        following_owner,
        preceding_owner,
        enclosing_owner,
        token_after_owner,
        token_before_owner,
    ]
    .into_iter()
    .flatten()
    .find_map(|owner| {
        promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::DependencyItem)
    })?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);

    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Attach one following-bound seam comment as one line-prefix comment.
fn attach_following_binding_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    token_after_span: Option<Span>,
) -> Option<CommentAttachment> {
    if !seam.seam_binds_right || seam.token_after_prefers_preceding {
        return None;
    }

    let target_owner = following_owner?;
    let target_owner = token_after_span
        .map(|span| promote_owner_by_shared_start(tree, parents, target_owner, span.start))
        .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Prepared ctx for one end-of-line seam comment.
struct EndOfLineCommentContext<'a, 'ctx> {
    /// The seam ctx.
    ctx: &'a CommentSeamContext<'ctx>,
    /// The seam facts.
    seam: &'a CommentSeamData,
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// Owner before the seam.
    preceding_owner: Option<u32>,
    /// Owner after the seam.
    following_owner: Option<u32>,
    /// Owner after the seam with token fallback.
    following_owner_with_token_after_fallback: Option<u32>,
    /// Span before the seam.
    token_before_span: Option<Span>,
    /// Span after the seam.
    token_after_span: Option<Span>,
    /// Owner before semicolon with token fallback.
    preceding_owner_with_semicolon_fallback: Option<u32>,
    /// Enclosing owner.
    enclosing_owner: Option<u32>,
    /// Same-line line comment classification.
    is_same_line_line_comment: bool,
    /// Same-line trailing block comment classification.
    is_same_line_trailing_block_comment: bool,
    /// Line-leading semicolon classification.
    semicolon_is_line_leading: bool,
    /// Line-leading semicolon classification for token after comment.
    semicolon_after_is_line_leading: bool,
}

/// Build one end-of-line seam comment ctx.
fn build_end_of_line_comment_ctx<'a, 'ctx>(
    ctx: &'a CommentSeamContext<'ctx>,
    seam: &'a CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> EndOfLineCommentContext<'a, 'ctx> {
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;
    let following_owner_with_token_after_fallback =
        following_owner_with_token_after_fallback(ctx.tree, ctx, following_owner);
    let token_before_span = ctx.token_before_span.map(|token| token.span);
    let token_after_span = ctx.token_after_span.map(|token| token.span);
    let preceding_owner_with_semicolon_fallback =
        preceding_owner_with_non_newline_token_before_fallback(ctx.tree, ctx, preceding_owner);
    let enclosing_owner = comment_enclosing_owner(ctx, enclosing_owner_cache);
    let is_same_line_line_comment = seam.comment_is_line && !seam.has_leading_newline;
    let is_same_line_trailing_block_comment =
        seam.comment_is_star && !seam.has_leading_newline && seam.has_trailing_newline;
    let semicolon_is_line_leading = seam_has_line_leading_semicolon_before_comment(ctx, seam);
    let semicolon_after_is_line_leading = seam_has_line_leading_semicolon_after_comment(ctx, seam);

    EndOfLineCommentContext {
        ctx,
        seam,
        tree: ctx.tree,
        parents: ctx.parents,
        preceding_owner,
        following_owner,
        following_owner_with_token_after_fallback,
        token_before_span,
        token_after_span,
        preceding_owner_with_semicolon_fallback,
        enclosing_owner,
        is_same_line_line_comment,
        is_same_line_trailing_block_comment,
        semicolon_is_line_leading,
        semicolon_after_is_line_leading,
    }
}

/// Run one ordered end-of-line handler sequence.
fn run_end_of_line_comment_handlers(
    ctx: &EndOfLineCommentContext<'_, '_>,
    handlers: &[fn(&EndOfLineCommentContext<'_, '_>) -> Option<CommentAttachment>],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Attach empty-statement semicolon ownership for end-of-line comments.
fn attach_end_of_line_empty_statement_semicolon_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_before_empty_statement_semicolon(
        ctx.tree,
        ctx.parents,
        ctx.ctx,
        ctx.seam,
        ctx.preceding_owner_with_semicolon_fallback,
        ctx.following_owner_with_token_after_fallback,
    )
}

/// Attach empty-if ownership for end-of-line comments.
fn attach_end_of_line_empty_if_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_empty_if_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.preceding_owner_with_semicolon_fallback,
        ctx.enclosing_owner,
        ctx.token_before_span,
        ctx.is_same_line_line_comment,
    )
}

/// Attach trailing-comma close-brace ownership for end-of-line comments.
fn attach_end_of_line_trailing_comma_close_brace_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_trailing_comma_close_brace_comment(
        ctx.tree,
        ctx.parents,
        ctx.ctx,
        ctx.seam,
        ctx.preceding_owner,
        ctx.is_same_line_line_comment,
    )
}

/// Attach dependency-item separator ownership for end-of-line comments.
fn attach_end_of_line_dependency_item_separator_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_after_dependency_item_separator_comma_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.ctx,
        ctx.seam,
        ctx.preceding_owner,
        ctx.is_same_line_line_comment,
    )
}

/// Attach dependency-item pre-separator ownership for end-of-line comments.
fn attach_end_of_line_dependency_item_before_separator_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_before_dependency_item_separator_comma_comment(
        ctx.tree,
        ctx.parents,
        ctx.ctx,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.is_same_line_line_comment,
        ctx.is_same_line_trailing_block_comment,
    )
}

/// Attach inline-before-semicolon ownership for end-of-line comments.
fn attach_end_of_line_inline_before_semicolon_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_inline_comment_before_semicolon(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.semicolon_after_is_line_leading,
        ctx.preceding_owner,
        ctx.token_before_span,
        ctx.is_same_line_line_comment || ctx.is_same_line_trailing_block_comment,
    )
}

/// Attach line-leading-semicolon ownership for end-of-line comments.
fn attach_end_of_line_line_leading_semicolon_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_after_line_leading_semicolon_comment(
        ctx.tree,
        ctx.ctx,
        ctx.seam,
        ctx.following_owner_with_token_after_fallback,
        ctx.is_same_line_line_comment || ctx.is_same_line_trailing_block_comment,
    )
}

/// Attach semicolon-terminated statement ownership for end-of-line comments.
fn attach_end_of_line_semicolon_terminated_statement_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_after_semicolon_terminated_statement_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.semicolon_is_line_leading,
        ctx.preceding_owner_with_semicolon_fallback,
        ctx.token_before_span,
        ctx.is_same_line_line_comment || ctx.is_same_line_trailing_block_comment,
    )
}

/// Attach binary-operator tail ownership for end-of-line comments.
fn attach_end_of_line_binary_operator_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_after_binary_operator_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.enclosing_owner,
        ctx.following_owner,
        ctx.token_after_span,
        ctx.is_same_line_line_comment,
    )
}

/// Attach inline break and continue ownership for end-of-line comments.
fn attach_end_of_line_inline_break_or_continue_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_inline_break_or_continue_comment(ctx.tree, ctx.ctx, ctx.is_same_line_line_comment)
}

/// Attach template interpolation ownership for end-of-line comments.
fn attach_end_of_line_template_interpolation_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_template_interpolation_open_brace_line_comment(
        ctx.tree,
        ctx.ctx,
        ctx.seam,
        ctx.following_owner_with_token_after_fallback,
        ctx.is_same_line_line_comment,
    )
}

/// Attach ternary-colon trailing ownership for end-of-line comments.
fn attach_end_of_line_after_ternary_colon_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_after_ternary_colon_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.enclosing_owner,
        ctx.token_before_span,
        ctx.is_same_line_line_comment,
    )
}

/// Attach ternary-colon prefix-star ownership for end-of-line comments.
fn attach_end_of_line_before_ternary_colon_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_before_ternary_colon_comment(
        ctx.tree,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner,
    )
}

/// Attach tree-expression close-brace ownership for end-of-line comments.
fn attach_end_of_line_tree_expression_close_brace_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_before_tree_expression_close_brace_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.preceding_owner,
        ctx.is_same_line_line_comment,
    )
}

/// Attach argument close-brace ownership for end-of-line comments.
fn attach_end_of_line_argument_close_brace_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_before_argument_close_brace_comment(
        ctx.tree,
        ctx.seam,
        ctx.preceding_owner,
        ctx.enclosing_owner,
        ctx.is_same_line_line_comment,
    )
}

/// Attach parenthesized tree-head ownership for end-of-line comments.
fn attach_end_of_line_parenthesized_tree_head_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_parenthesized_tree_head_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.following_owner_with_token_after_fallback,
        ctx.token_after_span,
        ctx.is_same_line_line_comment,
    )
}

/// Attach call-argument head ownership for end-of-line comments.
fn attach_end_of_line_call_argument_head_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_call_argument_head_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.token_before_span,
        ctx.seam,
        ctx.is_same_line_line_comment,
    )
}

/// Attach call-callee boundary ownership for end-of-line comments.
fn attach_end_of_line_call_callee_boundary_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_call_callee_line_comment(
        ctx.tree,
        ctx.seam,
        ctx.enclosing_owner,
        ctx.is_same_line_line_comment,
    )
}

/// Attach dependency-item alias ownership for end-of-line comments.
fn attach_end_of_line_dependency_item_alias_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_dependency_item_alias_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.ctx,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.enclosing_owner,
        ctx.is_same_line_line_comment,
    )
}

/// Attach following-binding ownership for end-of-line comments.
fn attach_end_of_line_following_binding_comment(
    ctx: &EndOfLineCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    attach_following_binding_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.following_owner,
        ctx.token_after_span,
    )
}

/// Attach end-of-line comments and terminal seam comments.
pub(crate) fn attach_end_of_line_comment(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.has_trailing_newline && ctx.token_after.is_some() {
        return None;
    }

    let ctx = build_end_of_line_comment_ctx(ctx, seam, enclosing_owner_cache, owners);

    run_end_of_line_comment_handlers(
        &ctx,
        &[
            attach_end_of_line_empty_statement_semicolon_comment,
            attach_end_of_line_empty_if_comment,
            attach_end_of_line_trailing_comma_close_brace_comment,
            attach_end_of_line_dependency_item_separator_comment,
            attach_end_of_line_dependency_item_before_separator_comment,
            attach_end_of_line_inline_before_semicolon_comment,
            attach_end_of_line_line_leading_semicolon_comment,
            attach_end_of_line_semicolon_terminated_statement_comment,
            attach_end_of_line_binary_operator_comment,
            attach_end_of_line_inline_break_or_continue_comment,
            attach_end_of_line_template_interpolation_comment,
            attach_end_of_line_after_ternary_colon_comment,
            attach_end_of_line_before_ternary_colon_comment,
            attach_end_of_line_tree_expression_close_brace_comment,
            attach_end_of_line_argument_close_brace_comment,
            attach_end_of_line_parenthesized_tree_head_comment,
            attach_end_of_line_call_argument_head_comment,
            attach_end_of_line_call_callee_boundary_comment,
            attach_end_of_line_dependency_item_alias_comment,
            attach_end_of_line_following_binding_comment,
        ],
    )
}
