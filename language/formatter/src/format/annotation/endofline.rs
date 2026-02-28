use destack_ast::{
    AnnotationPosition, Argument, Expression, IfKind, LocalNodeId, NodeParentIndex, NodeTree,
    NodeType, TokenType, UnaryOperator,
};
use destack_source::Span;

use super::attachment::{
    attach_line_comment_after_ternary_colon, attach_star_comment_before_ternary_colon,
    attach_trailing_comma_close_brace_property_line_comment,
    following_owner_with_token_after_fallback, if_expression_then_owner_without_else,
    promote_owner_to_tree_expression_parent, try_attach_comment_before_empty_statement_semicolon,
};
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner,
    seam_is_template_interpolation_open_brace,
};
use super::ownership::{
    find_owner_at_or_after_token, find_preferred_owner_starting_at,
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_by_shared_start,
    promote_owner_to_nearest_statement_boundary, promote_owner_to_node_type_ancestor,
};
use super::semicolon::preceding_owner_with_non_newline_token_fallback;

/// Return whether one token is a closing delimiter token.
#[inline]
fn token_type_is_close_delimiter(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::CloseBrace | TokenType::CloseBracket | TokenType::CloseParenthesis
    )
}

/// Attach one close-delimiter line comment before a standalone semicolon to the following owner.
fn attach_close_delimiter_semicolon_line_comment(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner_with_token_fallback: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam
            .token_before_type
            .is_some_and(token_type_is_close_delimiter)
        || !seam.token_after_is(TokenType::Semicolon)
    {
        return None;
    }

    let owner_after_semicolon = context.token_after.and_then(|token_after_index| {
        find_owner_at_or_after_token(
            tree,
            context.semantic_tokens,
            token_after_index.saturating_add(1),
        )
    });
    let target_owner = owner_after_semicolon.or(following_owner_with_token_fallback)?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
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

/// Attach one same-line import or export specifier separator comment to the following dependency item.
fn attach_after_dependency_item_separator_comma_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_before_is(TokenType::Comma) {
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

/// Attach one same-line dependency-item comment before a separator comma.
fn attach_before_dependency_item_separator_comma_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
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

    let token_before_owner = context
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

/// Attach one same-line line comment before one semicolon to the preceding expression boundary.
fn attach_before_semicolon_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam.token_after_is(TokenType::Semicolon)
        || seam.token_before_is(TokenType::Semicolon)
    {
        return None;
    }

    let target_node = preceding_owner?;
    let target_node =
        normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
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

/// Attach one same-line interpolation-head line comment to the interpolation owner.
fn attach_template_interpolation_open_brace_line_comment(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner_with_token_fallback: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam_is_template_interpolation_open_brace(context, seam) {
        return None;
    }

    let target_node = following_owner_with_token_fallback?;
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
    };
    Some((Some(value_id.id), AnnotationPosition::LinePostfixBoundary))
}

/// Attach one same-line line comment after `(` before one tree-expression head.
fn attach_parenthesized_tree_head_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner_with_token_fallback: Option<u32>,
    token_after_span: Option<Span>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment
        || !seam.token_before_is(TokenType::OpenParenthesis)
        || !seam.token_after_is(TokenType::LessThan)
    {
        return None;
    }

    let target_owner = following_owner_with_token_fallback?;
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
    seam: &CommentSeamData,
    enclosing_owner: Option<u32>,
    following_owner_with_token_fallback: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_before_is(TokenType::OpenParenthesis) {
        return None;
    }

    let Some(enclosing_owner) = enclosing_owner else {
        return None;
    };
    if tree.get_node_type(enclosing_owner) != NodeType::Expression {
        return None;
    }

    let enclosing_expression_id = LocalNodeId::<Expression>::new(enclosing_owner);
    if !matches!(
        tree.get(enclosing_expression_id),
        Expression::Call { .. } | Expression::New { .. }
    ) {
        return None;
    }

    let target_owner = following_owner_with_token_fallback?;
    let target_owner =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Argument)
            .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Attach one same-line line comment after `as` in import or export items to the dependency item.
fn attach_dependency_item_alias_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_before_is_keyword(CommentSeamKeyword::As) {
        return None;
    }

    let token_before_owner = context.token_before_span.and_then(|token| {
        find_preferred_owner_starting_at(tree, token.span)
            .or_else(|| find_smallest_owner_enclosing_token(tree, token.span))
    });
    let token_after_owner = context.token_after_span.and_then(|token| {
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

/// Attach one same-line line comment between callee and `(` to the full call expression.
fn attach_call_callee_head_line_comment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    enclosing_owner: Option<u32>,
    is_same_line_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_line_comment || !seam.token_after_is(TokenType::OpenParenthesis) {
        return None;
    }

    let target_owner = enclosing_owner?;
    if tree.get_node_type(target_owner) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(target_owner);
    if !matches!(
        tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. }
    ) {
        return None;
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
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
        preceding_owner_with_non_newline_token_fallback(tree, context, preceding_owner);
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

    // import and export specifier separator ownership
    if let Some(attachment) = attach_after_dependency_item_separator_comma_line_comment(
        tree,
        parents,
        context,
        seam,
        preceding_owner,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // import and export specifier comments before separator commas stay on dependency items
    if let Some(attachment) = attach_before_dependency_item_separator_comma_comment(
        tree,
        parents,
        context,
        seam,
        preceding_owner,
        following_owner,
        is_same_line_line_comment,
        is_same_line_trailing_block_comment,
    ) {
        return Some(attachment);
    }

    // close-delimiter comments before standalone semicolons belong to the following statement
    if let Some(attachment) = attach_close_delimiter_semicolon_line_comment(
        tree,
        context,
        seam,
        following_owner_with_token_fallback,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // same-line comments before semicolons stay with the preceding expression
    if let Some(attachment) = attach_before_semicolon_line_comment(
        tree,
        parents,
        seam,
        preceding_owner,
        token_before_span,
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

    // operator-tail comments in multiline binary expressions belong to the shared binary owner
    if let Some(attachment) = attach_after_binary_operator_line_comment(
        tree,
        parents,
        seam,
        enclosing_owner,
        following_owner,
        token_after_span,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // inline break or continue ownership
    if let Some(attachment) =
        attach_inline_break_or_continue_comment(tree, context, is_same_line_line_comment)
    {
        return Some(attachment);
    }

    // template interpolation `${` line-comment ownership
    if let Some(attachment) = attach_template_interpolation_open_brace_line_comment(
        tree,
        context,
        seam,
        following_owner_with_token_fallback,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // ternary colon trailing ownership
    if let Some(attachment) = attach_after_ternary_colon_comment(
        tree,
        parents,
        seam,
        preceding_owner,
        following_owner,
        enclosing_owner,
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

    // parenthesized tree-expression head ownership
    if let Some(attachment) = attach_parenthesized_tree_head_line_comment(
        tree,
        parents,
        seam,
        following_owner_with_token_fallback,
        token_after_span,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // call argument head line comment ownership
    if let Some(attachment) = attach_call_argument_head_line_comment(
        tree,
        parents,
        seam,
        enclosing_owner,
        following_owner_with_token_fallback,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // import and export alias comments after `as` belong to dependency item heads
    if let Some(attachment) = attach_dependency_item_alias_line_comment(
        tree,
        parents,
        context,
        seam,
        preceding_owner,
        following_owner,
        enclosing_owner,
        is_same_line_line_comment,
    ) {
        return Some(attachment);
    }

    // call callee-head line comments belong to the full call expression
    if let Some(attachment) =
        attach_call_callee_head_line_comment(tree, seam, enclosing_owner, is_same_line_line_comment)
    {
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
