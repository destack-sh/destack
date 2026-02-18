use ast::{AnnotationPosition, NodeParentIndex, NodeTree, TokenType};
use destack_ast as ast;

use super::rule::promote_rhs_expression_owner;
use super::seam::{
    CommentSeamContext, CommentSeamFacts, CommentSeamRuleState, resolve_comment_seam_owner,
};

/// Resolve assignment seam comment rules.
pub(super) fn resolve_comment_assignment_rules(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    state: &mut CommentSeamRuleState,
    right_owner: Option<u32>,
) -> Option<(Option<u32>, AnnotationPosition)> {
    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_line = facts.comment_is_line;
    let token_before_is_assign = facts.token_before_is(TokenType::Assign);
    let token_after_span = context.token_after_span;
    let token_before_span = context.token_before_span;

    // comments between assignment and rhs should bind to the rhs seam
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_assign
        && let Some(target_node) =
            right_owner.or_else(|| resolve_comment_seam_owner(context, state))
    {
        let target_node = promote_rhs_expression_owner(
            tree,
            parents,
            target_node,
            token_after_span.map(|token| token.span),
        );

        let comment_starts_on_assign_line = token_before_span.is_some_and(|before_token| {
            let before_line = context
                .file
                .get_position(before_token.span.start)
                .map_or(0, |position| position.0);
            let comment_line = context
                .file
                .get_position(context.trivia.span.start)
                .map_or(0, |position| position.0);
            before_line == comment_line
        });

        if comment_is_line && comment_starts_on_assign_line {
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // own-line comments between assignment and rhs stay on the rhs value region
    if has_leading_newline
        && token_before_is_assign
        && let Some(target_node) =
            right_owner.or_else(|| resolve_comment_seam_owner(context, state))
    {
        let target_node = promote_rhs_expression_owner(
            tree,
            parents,
            target_node,
            token_after_span.map(|token| token.span),
        );
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}
