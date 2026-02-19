use ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenSpan,
    TokenType,
};
use destack_ast as ast;
use destack_source::File;

use super::assignment::resolve_comment_assignment_rules;
use super::context::build_comment_attachment_setup;
use super::declaration::resolve_comment_declaration_rules;
use super::default::resolve_formatter_comment_trivia_default;
use super::expression::resolve_comment_expression_rules;
use super::index::FormatterTriviaOwnerIndex;
use super::owner::{
    find_smallest_owner_enclosing_range, lowest_common_owner_ancestor,
    normalize_formatter_trivia_target_owner,
};
use super::seam::{CommentSeamFacts, CommentSeamRuleState};
use super::statement::{
    resolve_comment_block_body_rules, resolve_comment_statement_prefix_rules,
    resolve_comment_statement_suffix_rules,
};
use super::token::{delimiters_match, is_close_delimiter_token, is_open_delimiter_token};

/// Resolve one comment trivia target owner and position from token seams.
pub(in super::super) fn resolve_formatter_comment_trivia_attachment(
    file: &File,
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    trivia: destack_ast::CommentTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
) -> (Option<u32>, AnnotationPosition) {
    let setup =
        build_comment_attachment_setup(file, tree, semantic_tokens, trivia, owner_index, parents);
    let context = setup.context;
    let left_owner = setup.left_owner;
    let right_owner = setup.right_owner;
    let token_before_span = context.token_before_span;
    let token_after_span = context.token_after_span;

    // delimiter interiors use container infix placement
    if let (Some(token_before_span), Some(token_after_span)) = (token_before_span, token_after_span)
    {
        if is_open_delimiter_token(token_before_span.token.ty)
            && is_close_delimiter_token(token_after_span.token.ty)
            && delimiters_match(token_before_span.token.ty, token_after_span.token.ty)
            && let Some(container_owner) = find_smallest_owner_enclosing_range(
                tree,
                token_before_span.span.start,
                token_after_span.span.end,
            )
        {
            let target_node = if tree.get_node_type(container_owner) == NodeType::Expression {
                let expression_id = LocalNodeId::<Expression>::new(container_owner);
                if let Expression::Block(block_id) = tree.get(expression_id) {
                    block_id.id
                } else {
                    normalize_formatter_trivia_target_owner(tree, container_owner)
                }
            } else {
                normalize_formatter_trivia_target_owner(tree, container_owner)
            };
            return (Some(target_node), AnnotationPosition::BlockInfix);
        }
    }

    // comments between parameter name and type belong to the whole parameter owner
    if token_after_span.is_some_and(|token| token.token.ty == TokenType::Colon)
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && let Some(owner) = lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
        && tree.get_node_type(owner) == NodeType::Parameter
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, owner);
        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    let facts = CommentSeamFacts::build(&context);
    let mut state = CommentSeamRuleState::default();

    if let Some(decision) = resolve_comment_expression_rules(
        tree,
        owner_index,
        parents,
        &context,
        &facts,
        &mut state,
        left_owner,
        right_owner,
    ) {
        return decision;
    }

    if let Some(decision) = resolve_comment_statement_prefix_rules(
        tree,
        parents,
        &context,
        &facts,
        &mut state,
        left_owner,
        right_owner,
    ) {
        return decision;
    }

    if let Some(decision) = resolve_comment_declaration_rules(
        tree,
        owner_index,
        parents,
        &context,
        &facts,
        left_owner,
        right_owner,
    ) {
        return decision;
    }

    if let Some(decision) =
        resolve_comment_statement_suffix_rules(tree, parents, &facts, left_owner, right_owner)
    {
        return decision;
    }

    if let Some(decision) =
        resolve_comment_assignment_rules(tree, parents, &context, &facts, &mut state, right_owner)
    {
        return decision;
    }

    if let Some(decision) = resolve_comment_block_body_rules(tree, &facts, right_owner) {
        return decision;
    }

    resolve_formatter_comment_trivia_default(&context, &facts, &mut state, left_owner, right_owner)
}
