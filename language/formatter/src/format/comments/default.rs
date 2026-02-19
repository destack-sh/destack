use ast::{AnnotationPosition, NodeParentIndex};
use destack_ast as ast;

use super::owner::{
    find_smallest_owner_enclosing_range, normalize_formatter_trivia_target_owner,
    promote_owner_by_shared_start,
};
use super::rule::normalize_owner_with_shared_end;
use super::seam::{
    CommentSeamContext, CommentSeamFacts, CommentSeamKeyword, CommentSeamRuleState,
    resolve_comment_seam_owner,
};

/// Resolve the default comment trivia rules after specialized seam cases.
pub(super) fn resolve_formatter_comment_trivia_default(
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    state: &mut CommentSeamRuleState,
    left_owner: Option<u32>,
    right_owner: Option<u32>,
) -> (Option<u32>, AnnotationPosition) {
    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let token_before = context.token_before;
    let token_after = context.token_after;

    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_multiline_star = facts.comment_is_multiline_star;
    let token_after_is_else = facts.token_after_is_keyword(CommentSeamKeyword::Else);
    let token_before_is_open_delimiter = facts
        .token_before_type
        .is_some_and(super::token::is_open_delimiter_token);
    let token_after_is_less_than = facts.token_after_is(ast::TokenType::LessThan);
    let token_after_prefers_left = facts.token_after_prefers_left;
    let seam_binds_right = facts.seam_binds_right;

    // own-line comments before `else` should stay between the previous branch and `else`
    if has_leading_newline
        && token_after_is_else
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // own-line comments inside parenthesized groups before `<` should stay with the right side
    if has_leading_newline
        && token_before_is_open_delimiter
        && token_after_is_less_than
        && let Some(target_node) = resolve_comment_seam_owner(context, state).or(right_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // own-line comments before separators and closers belong to the left owner
    if has_leading_newline
        && token_after_prefers_left
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // default own-line comment binding: right owner
    if has_leading_newline && let Some(target_node) = right_owner {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // inline multiline block comments before line-end should break to postfix blocks
    if comment_is_multiline_star
        && has_trailing_newline
        && !has_leading_newline
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // default trailing-line comment binding: left owner
    if has_trailing_newline || token_after.is_none() {
        if seam_binds_right
            && !token_after_prefers_left
            && let Some(target_node) = right_owner
        {
            let target_node = token_after_span
                .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
                .unwrap_or(target_node);
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::LinePrefix);
        }

        if let Some(target_node) = left_owner {
            let target_node =
                normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
            return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
        }
    }

    // same-line seams that precede separators and operators prefer left postfix
    if token_after_prefers_left && let Some(target_node) = left_owner {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return (Some(target_node), AnnotationPosition::LinePostfix);
    }

    // same-line seams prefer the right owner as line-prefix
    if let Some(target_node) = right_owner {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    // fallback to left owner as line-postfix
    if let Some(target_node) = left_owner {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return (Some(target_node), AnnotationPosition::LinePostfix);
    }

    // comment-only files can still anchor to one enclosing owner
    if token_before.is_none()
        && token_after.is_none()
        && let Some(target_node) = find_smallest_owner_enclosing_range(
            tree,
            context.trivia.span.start,
            context.trivia.span.end,
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    (None, AnnotationPosition::BlockInfix)
}
