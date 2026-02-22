use ast::{
    AnnotationPosition, Argument, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
};
use destack_ast as ast;

use crate::format::comments::owner::{
    find_smallest_owner_enclosing_range, find_smallest_owner_enclosing_token, is_block_like_owner,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_start, promote_owner_to_node_type_ancestor,
};
use crate::format::comments::seam::{
    CommentAttachmentDecision, CommentAttachmentOwners, CommentSeamContext, CommentSeamFacts,
    CommentSeamKeyword, CommentSeamOwnerCache, resolve_comment_seam_owner,
};

/// Resolve the default comment trivia rules after specialized seam cases.
pub(crate) fn attach_comment_default(
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> CommentAttachmentDecision {
    // own line cases
    if let Some(decision) = try_attach_own_line_comment(context, facts, seam_owner_cache, owners) {
        return decision;
    }

    // inline multiline block comments before line end
    if let Some(decision) = try_attach_inline_multiline_postfix_comment(context, facts, owners) {
        return decision;
    }

    // trailing line and terminal cases
    if let Some(decision) =
        try_attach_trailing_line_comment(context, facts, seam_owner_cache, owners)
    {
        return decision;
    }

    // same line default prefers right, then left
    if let Some(decision) = try_attach_same_line_default(context, facts, owners) {
        return decision;
    }

    // comment only files can still anchor to one enclosing owner
    if let Some(decision) = try_attach_comment_only_file_comment(context) {
        return decision;
    }

    (None, AnnotationPosition::BlockInfix)
}

/// Attach own line comments with dedicated own line rules.
fn try_attach_own_line_comment(
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if !facts.has_leading_newline {
        return None;
    }

    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let left_owner = owners.left;
    let right_owner = owners.right;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);

    // own line comments before nested else branches should stay on the if seam
    if facts.token_after_is_keyword(CommentSeamKeyword::Else) {
        let seam_owner = resolve_comment_seam_owner(context, seam_owner_cache);
        let seam_owner_is_nested_if = seam_owner.is_some_and(|owner| {
            if tree.get_node_type(owner) != NodeType::Expression {
                return false;
            }

            if !matches!(
                tree.get(LocalNodeId::<Expression>::new(owner)),
                Expression::If { .. }
            ) {
                return false;
            }

            let Some(parent_id) = parents.get_by_id(owner) else {
                return false;
            };
            if tree.get_node_type(parent_id) != NodeType::Expression {
                return false;
            }

            matches!(
                tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::If { .. }
            )
        });
        let right_owner_is_block =
            right_owner.is_some_and(|owner| is_block_like_owner(tree, owner));

        if seam_owner_is_nested_if
            && !right_owner_is_block
            && let Some(target_node) = seam_owner
        {
            let target_node =
                normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockPostfix));
        }

        if let Some(mut target_node) = right_owner.or(seam_owner).or(left_owner) {
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

            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            let position = if !facts.comment_is_line && !facts.comment_is_multiline_star {
                AnnotationPosition::LinePrefix
            } else {
                AnnotationPosition::BlockPrefix
            };
            return Some((Some(target_node), position));
        }
    }

    // own line comments inside parenthesized groups before < should stay with the right side
    let token_before_is_open_delimiter = facts
        .token_before_type
        .is_some_and(super::token::is_open_delimiter_token);
    if token_before_is_open_delimiter
        && facts.token_after_is(ast::TokenType::LessThan)
        && let Some(target_node) =
            right_owner.or_else(|| resolve_comment_seam_owner(context, seam_owner_cache))
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // own line comments before separators and closers belong to the left owner
    if facts.token_after_prefers_left
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        let position = if facts.comment_is_line {
            if facts.token_after_is(ast::TokenType::CloseBrace) {
                AnnotationPosition::BlockPostfix
            } else {
                AnnotationPosition::LinePostfixBoundary
            }
        } else {
            AnnotationPosition::BlockPostfix
        };
        return Some((Some(target_node), position));
    }

    // default own line comment binding: right owner
    if let Some(target_node) = right_owner {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}

/// Attach inline multiline block comments before line end.
fn try_attach_inline_multiline_postfix_comment(
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if !facts.comment_is_multiline_star || !facts.has_trailing_newline || facts.has_leading_newline
    {
        return None;
    }

    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let left_owner = owners.left?;

    let target_node = normalize_owner_with_shared_end(tree, parents, left_owner, token_before_span);
    Some((Some(target_node), AnnotationPosition::BlockPostfix))
}

/// Attach trailing line comments and terminal seam comments.
fn try_attach_trailing_line_comment(
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if !facts.has_trailing_newline && context.token_after.is_some() {
        return None;
    }

    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let left_owner = owners.left;
    let right_owner = owners.right;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);

    // trailing comments after empty if statements stay on the empty consequent
    if facts.comment_is_line
        && !facts.has_leading_newline
        && facts.token_before_is(ast::TokenType::Semicolon)
        && let Some(left_owner) = left_owner
    {
        let target_owner =
            normalize_owner_with_shared_end(tree, parents, left_owner, token_before_span);
        if let Some(then_owner) = if_expression_then_owner_without_else(tree, target_owner) {
            return Some((Some(then_owner), AnnotationPosition::LinePostfixBoundary));
        }
    }

    if facts.comment_is_line
        && !facts.has_leading_newline
        && facts.token_before_is(ast::TokenType::Semicolon)
        && let Some(seam_owner) = resolve_comment_seam_owner(context, seam_owner_cache)
        && let Some(then_owner) = if_expression_then_owner_without_else(tree, seam_owner)
    {
        return Some((Some(then_owner), AnnotationPosition::LinePostfixBoundary));
    }

    // line comments after trailing commas before `}` should stay on the enclosing member
    if facts.comment_is_line
        && !facts.has_leading_newline
        && facts.token_before_is(ast::TokenType::Comma)
        && facts.token_after_is(ast::TokenType::CloseBrace)
    {
        let target_node = context.token_before_span.and_then(|comma_token| {
            let search_start = comma_token.span.start.saturating_sub(1);
            (search_start < comma_token.span.start).then(|| {
                find_smallest_owner_enclosing_range(tree, search_start, comma_token.span.start)
            })?
        });
        let target_node = target_node.or(left_owner);
        let Some(target_node) = target_node else {
            return None;
        };

        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        if let Some(member_owner) =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Property)
        {
            let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
            return Some((Some(member_owner), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // trailing comments after inline break and continue statements stay on the control statement
    if facts.comment_is_line
        && !facts.has_leading_newline
        && let Some(token_before_span) = context.token_before_span
        && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token_before_span.span)
        && tree.get_node_type(target_node) == NodeType::Expression
    {
        let target_expression = LocalNodeId::<Expression>::new(target_node);
        if matches!(
            tree.get(target_expression),
            Expression::Break { .. } | Expression::Continue { .. }
        ) {
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // line comments after ternary `:` stay with the consequent branch boundary
    if facts.comment_is_line
        && !facts.has_leading_newline
        && facts.token_before_is(ast::TokenType::Colon)
        && !facts.token_before_is_return_type_colon
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // block comments immediately before ternary `:` stay on the branch seam
    if facts.comment_is_star
        && facts.token_after_is(ast::TokenType::Colon)
        && !facts.token_before_is_return_type_colon
        && let Some(left_target_node) = left_owner
        && tree.get_node_type(left_target_node) == NodeType::Expression
    {
        let right_prefers_tree_expression = right_owner.is_some_and(|target_node| {
            tree.get_node_type(target_node) == NodeType::Expression
                && matches!(
                    tree.get(LocalNodeId::<Expression>::new(target_node)),
                    Expression::TreeExpression { .. }
                )
        });
        if right_prefers_tree_expression && let Some(target_node) = right_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        let target_expression = LocalNodeId::<Expression>::new(left_target_node);
        if let Expression::Parenthesized { expression } = tree.get(target_expression) {
            let target_node = normalize_formatter_trivia_target_owner(tree, expression.id);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, left_target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // line comments before tree-expression container `}` stay on the container value expression
    if facts.comment_is_line
        && !facts.has_leading_newline
        && facts.token_after_is(ast::TokenType::CloseBrace)
        && let Some(left_expression_owner) = left_owner
            .filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)
            .filter(|owner| {
                parents
                    .get_by_id(*owner)
                    .is_some_and(|parent_id| tree.get_node_type(parent_id) == NodeType::Argument)
            })
    {
        return Some((
            Some(left_expression_owner),
            AnnotationPosition::LinePostfixBoundary,
        ));
    }

    if facts.comment_is_line
        && !facts.has_leading_newline
        && facts.token_after_is(ast::TokenType::CloseBrace)
        && let Some(target_node) = left_owner
            .filter(|owner| tree.get_node_type(*owner) == NodeType::Argument)
            .or_else(|| {
                resolve_comment_seam_owner(context, seam_owner_cache)
                    .filter(|owner| tree.get_node_type(*owner) == NodeType::Argument)
            })
    {
        let argument_id = LocalNodeId::<Argument>::new(target_node);
        let value_id = match tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => *value,
        };
        return Some((Some(value_id.id), AnnotationPosition::LinePostfixBoundary));
    }

    // comments after most : seams stay with the left segment
    let colon_prefers_left = facts.token_before_is(ast::TokenType::Colon)
        && !facts.token_before_is_return_type_colon
        && facts.comment_is_line;

    if facts.seam_binds_right
        && !facts.token_after_prefers_left
        && !colon_prefers_left
        && let Some(target_node) = right_owner
    {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments after one member semicolon belong to the member, not the value expression
    if facts.token_before_is(ast::TokenType::Semicolon)
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        if let Some(member_owner) =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Member)
        {
            let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
            return Some((Some(member_owner), AnnotationPosition::LinePostfixBoundary));
        }
    }

    if let Some(target_node) = left_owner {
        let should_keep_terminal_left_owner = facts
            .token_after_is(ast::TokenType::CloseParenthesis)
            && facts.token_before_is(ast::TokenType::Literal)
            && !facts.token_before_is(ast::TokenType::Comma);
        let target_node = if should_keep_terminal_left_owner {
            target_node
        } else {
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span)
        };
        let position = if should_keep_terminal_left_owner {
            AnnotationPosition::LinePostfix
        } else {
            AnnotationPosition::LinePostfixBoundary
        };
        return Some((Some(target_node), position));
    }

    None
}

/// Return the then-branch owner id for one if-expression owner that has no else branch.
fn if_expression_then_owner_without_else(tree: &NodeTree, owner: u32) -> Option<u32> {
    if tree.get_node_type(owner) != NodeType::Expression {
        return None;
    }

    let seam_expression = LocalNodeId::<Expression>::new(owner);
    if let Expression::If {
        then_expression,
        else_expression,
        ..
    } = tree.get(seam_expression)
        && else_expression.is_none()
    {
        return Some(then_expression.id);
    }

    None
}

/// Attach same line comments when trailing rules do not apply.
fn try_attach_same_line_default(
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let left_owner = owners.left;
    let right_owner = owners.right;

    // same line seams that precede separators and operators prefer left postfix
    if facts.token_after_prefers_left
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // same line seams prefer the right owner as line prefix
    if let Some(target_node) = right_owner {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // default to left owner as line postfix
    if let Some(target_node) = left_owner {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    None
}

/// Attach comments when the file contains only comments and no surrounding tokens.
fn try_attach_comment_only_file_comment(
    context: &CommentSeamContext<'_>,
) -> Option<CommentAttachmentDecision> {
    if context.token_before.is_some() || context.token_after.is_some() {
        return None;
    }

    let tree = context.tree;
    let target_node = find_smallest_owner_enclosing_range(
        tree,
        context.trivia.span.start,
        context.trivia.span.end,
    )?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::BlockPrefix))
}
