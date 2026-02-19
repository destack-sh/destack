use ast::{
    AnnotationPosition, Block, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenType,
};
use destack_ast as ast;

use super::owner::{
    find_smallest_owner_enclosing_token, lowest_common_owner_ancestor,
    normalize_formatter_trivia_target_owner, promote_owner_to_node_type_ancestor,
    resolve_block_leading_comment_target,
};
use super::rule::normalize_owner_with_shared_end;
use super::seam::{
    CommentAttachmentDecision, CommentAttachmentOwners, CommentSeamContext, CommentSeamFacts,
    CommentSeamOwnerCache, resolve_comment_seam_owner,
};

/// Resolve statement-prefix seam comment rules.
pub(super) fn try_attach_comment_statement_prefix(
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
    let token_after_is_case_or_default = facts.token_after_is_case_or_default();
    let token_after_is_semicolon = facts.token_after_is(TokenType::Semicolon);
    let token_after_span = context.token_after_span;
    let token_before_span = context.token_before_span.map(|token| token.span);

    // own-line comments before semicolon guards stay with the previous statement seam
    if has_leading_newline && token_after_is_semicolon {
        if let Some(target_node) = left_owner {
            let target_node =
                normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        if let Some(mut target_node) = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
        {
            if tree.get_node_type(target_node) != NodeType::Expression
                && let Some(expression_target) = promote_owner_to_node_type_ancestor(
                    tree,
                    parents,
                    target_node,
                    NodeType::Expression,
                )
            {
                target_node = expression_target;
            }

            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }
    }

    // own-line comments before switch case labels should attach to the first case expression
    if has_leading_newline
        && token_after_is_case_or_default
        && let Some(target_node) =
            resolve_comment_seam_owner(context, seam_owner_cache).or(right_owner)
    {
        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if let Expression::Match { cases, .. } = tree.get(expression_id)
                && let Some(first_case) = cases.first().copied()
            {
                return Some((Some(first_case.id), AnnotationPosition::BlockPrefix));
            }
        }

        if tree.get_node_type(target_node) == NodeType::Block {
            let block_id = LocalNodeId::<Block>::new(target_node);
            let (target_node, position) = resolve_block_leading_comment_target(tree, block_id);
            return Some((Some(target_node), position));
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}

/// Resolve statement-suffix seam comment rules.
pub(super) fn try_attach_comment_statement_suffix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let left_owner = owners.left;
    let right_owner = owners.right;
    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_line = facts.comment_is_line;

    let token_after_is_case_or_default = facts.token_after_is_case_or_default();
    let token_after_is_close_parenthesis = facts.token_after_is(TokenType::CloseParenthesis);

    let token_before_is_comma = facts.token_before_is(TokenType::Comma);
    let token_before_is_control_head_close_paren = facts.token_before_is_control_head_close_paren;
    let token_before_is_return_type_colon = facts.token_before_is_return_type_colon;

    // trailing line comments after control heads should stay before the body statement
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_control_head_close_paren
        && !token_after_is_case_or_default
        && comment_is_line
    {
        if let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
            && let Some(shared_owner) =
                lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
            && tree.get_node_type(shared_owner) == NodeType::Expression
        {
            let shared_expression = LocalNodeId::<Expression>::new(shared_owner);
            match tree.get(shared_expression) {
                Expression::If {
                    then_expression, ..
                } => {
                    if matches!(tree.get(*then_expression), Expression::Block(_)) {
                        let then_block_expression = *then_expression;
                        let block_id =
                            if let Expression::Block(block_id) = tree.get(then_block_expression) {
                                *block_id
                            } else {
                                unreachable!()
                            };
                        let (target_node, position) =
                            resolve_block_leading_comment_target(tree, block_id);
                        return Some((Some(target_node), position));
                    }
                    return Some((Some(then_expression.id), AnnotationPosition::BlockPrefix));
                }
                Expression::While { body, .. }
                | Expression::ForEach { body, .. }
                | Expression::For { body, .. }
                | Expression::Loop { body } => {
                    let (target_node, position) = resolve_block_leading_comment_target(tree, *body);
                    return Some((Some(target_node), position));
                }
                _ => {}
            }
        }

        if let Some(target_node) = right_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }
    }

    // return type seam comments should stay between `:` and the return type
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_return_type_colon
        && comment_is_line
        && let Some(target_node) = right_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // parameter trailing comments before `)` should stay attached to the parameter
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_comma
        && token_after_is_close_parenthesis
        && comment_is_line
        && let Some(target_node) = left_owner
    {
        let target_node =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Parameter)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // trailing comments after callback arguments should stay with the callback argument
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_comma
        && !token_after_is_close_parenthesis
        && comment_is_line
        && let Some(target_node) = left_owner
    {
        let target_node =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Argument)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Resolve block body seam comment rules.
pub(super) fn try_attach_comment_block_body(
    tree: &NodeTree,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let right_owner = owners.right;
    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let token_after_is_open_brace = facts.token_after_is(TokenType::OpenBrace);

    // comments between method signatures and opening braces should stay inside the body
    if !has_leading_newline
        && has_trailing_newline
        && token_after_is_open_brace
        && let Some(target_node) = right_owner
    {
        if tree.get_node_type(target_node) == NodeType::Block {
            let block_id = LocalNodeId::<Block>::new(target_node);
            let (target_node, position) = resolve_block_leading_comment_target(tree, block_id);
            return Some((Some(target_node), position));
        }

        if tree.get_node_type(target_node) == NodeType::Declaration {
            return Some((Some(target_node), AnnotationPosition::BlockInfix));
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}
