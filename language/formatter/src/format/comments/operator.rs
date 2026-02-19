use ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenType,
};
use destack_ast as ast;

use super::owner::{
    normalize_formatter_trivia_target_owner, promote_owner_by_shared_start,
    promote_owner_to_satisfies_expression_ancestor,
};
use super::seam::{
    CommentAttachmentDecision, CommentAttachmentOwners, CommentSeamContext, CommentSeamFacts,
    CommentSeamKeyword, CommentSeamOwnerCache, resolve_comment_seam_owner,
};

/// Resolve expression operator seam comment rules.
pub(super) fn try_attach_comment_expression_operator(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let left_owner = owners.left;
    let right_owner = owners.right;
    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_line = facts.comment_is_line;
    let comment_is_star = facts.comment_is_star;
    let comment_is_multiline_star = facts.comment_is_multiline_star;

    let token_after_is_maybe = facts.token_after_is(TokenType::Maybe);
    let token_after_is_as = facts.token_after_is_keyword(CommentSeamKeyword::As);
    let token_after_is_satisfies = facts.token_after_is_keyword(CommentSeamKeyword::Satisfies);
    let token_after_is_const = facts.token_after_is_keyword(CommentSeamKeyword::Const);
    let token_after_is_elementwise_operator = matches!(
        facts.token_after_type,
        Some(TokenType::ElementwiseAnd | TokenType::ElementwiseOr | TokenType::ElementwiseXor)
    );

    let token_before_is_as = facts.token_before_is_keyword(CommentSeamKeyword::As);
    let token_before_is_satisfies = facts.token_before_is_keyword(CommentSeamKeyword::Satisfies);
    let token_before_is_less_than = facts.token_before_is(TokenType::LessThan);
    let token_before_is_elementwise_operator = matches!(
        facts.token_before_type,
        Some(TokenType::ElementwiseAnd | TokenType::ElementwiseOr | TokenType::ElementwiseXor)
    );
    let token_before_is_elementwise_or =
        matches!(facts.token_before_type, Some(TokenType::ElementwiseOr));
    let seam_owner = resolve_comment_seam_owner(context, seam_owner_cache);

    // seam comments before `as` and `satisfies` stay with the asserted left expression
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && (token_after_is_as || token_after_is_satisfies)
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // optional call line comments should stay on the full optional expression
    if !has_leading_newline
        && has_trailing_newline
        && token_after_is_maybe
        && comment_is_line
        && let Some(target_node) = resolve_comment_seam_owner(context, seam_owner_cache)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // optional call block comments should stay on the left call segment
    // and preserve tight `/* comment */?.` seams
    if !has_leading_newline
        && !has_trailing_newline
        && token_after_is_maybe
        && comment_is_star
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // trailing line comments before type and bitwise separators stay with the left operand
    if !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && token_after_is_elementwise_operator
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // comments after `as` should resolve to the cast expression seam
    if token_before_is_as
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && let Some(target_node) = resolve_comment_seam_owner(context, seam_owner_cache)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // line comments after `satisfies` only move to rhs prefixes for multi-argument type paths
    if token_before_is_satisfies && !has_leading_newline && has_trailing_newline && comment_is_line
    {
        if let Some(right_target) = right_owner {
            let mut prefix_target = None;

            if tree.get_node_type(right_target) == NodeType::Expression {
                let right_expression = LocalNodeId::<Expression>::new(right_target);
                if matches!(
                    tree.get(right_expression),
                    Expression::Path {
                        static_arguments: Some(static_arguments),
                        ..
                    } if static_arguments.len() > 1
                ) {
                    prefix_target = Some(right_target);
                }
            }

            if let Some(prefix_target) = prefix_target {
                let prefix_target = normalize_formatter_trivia_target_owner(tree, prefix_target);
                return Some((Some(prefix_target), AnnotationPosition::LinePrefix));
            }
        }

        if let Some(target_node) =
            resolve_comment_seam_owner(context, seam_owner_cache).or(left_owner)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // line comments after `<` in satisfies rhs type arguments stay with the rhs type
    if token_before_is_less_than
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && resolve_comment_seam_owner(context, seam_owner_cache)
            .or(left_owner)
            .and_then(|target_node| {
                promote_owner_to_satisfies_expression_ancestor(tree, parents, target_node)
            })
            .is_some()
    {
        if let Some(right_target) = right_owner {
            let right_target = normalize_formatter_trivia_target_owner(tree, right_target);
            return Some((Some(right_target), AnnotationPosition::LinePrefix));
        }
    }

    // line comments after type and bitwise operators should stay with the rhs operand
    if token_before_is_elementwise_operator
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
    {
        if token_before_is_elementwise_or && let Some(target_node) = left_owner.or(seam_owner) {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        let Some(target_node) = right_owner else {
            return None;
        };
        let target_node = context
            .token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // multiline comments between `as` and `const` stay after the assertion
    if token_before_is_as
        && token_after_is_const
        && !has_leading_newline
        && comment_is_multiline_star
        && let Some(target_node) = resolve_comment_seam_owner(context, seam_owner_cache)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}
