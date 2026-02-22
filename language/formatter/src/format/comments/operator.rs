use ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenType,
};
use destack_ast as ast;

use crate::format::comments::owner::{
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    promote_owner_by_shared_start, promote_owner_to_satisfies_expression_ancestor,
    promote_rhs_expression_owner,
};
use crate::format::comments::seam::{
    CommentAttachmentDecision, CommentAttachmentOwners, CommentSeamContext, CommentSeamFacts,
    CommentSeamKeyword, CommentSeamOwnerCache, resolve_comment_seam_owner,
};

/// Promote one owner to the nearest elementwise binary expression ancestor.
fn promote_owner_to_elementwise_binary_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if matches!(
                tree.get(expression_id),
                Expression::Binary {
                    operator: ast::BinaryOperator::ElementwiseAnd
                        | ast::BinaryOperator::ElementwiseOr
                        | ast::BinaryOperator::ElementwiseXor,
                    ..
                }
            ) {
                return Some(node_id);
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Descend transparent wrappers to the innermost owned expression.
fn descend_owner_through_transparent_expression_wrappers(tree: &NodeTree, owner_id: u32) -> u32 {
    let mut current_id = owner_id;

    loop {
        if tree.get_node_type(current_id) != NodeType::Expression {
            return current_id;
        }

        let expression_id = LocalNodeId::<Expression>::new(current_id);
        let next_id = match tree.get(expression_id) {
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                Some(expression.id)
            }
            _ => None,
        };

        let Some(next_id) = next_id else {
            return current_id;
        };
        current_id = next_id;
    }
}

/// Return whether one owner is a mapped type expression.
fn owner_is_type_mapped_expression(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(tree.get(expression_id), Expression::TypeMapped { .. })
}

/// Return whether one owner has a mapped-type expression ancestor.
fn owner_has_type_mapped_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if owner_is_type_mapped_expression(tree, node_id) {
            return true;
        }

        current_id = parents.get_by_id(node_id);
    }

    false
}

/// Resolve assignment seam comment rules.
pub(crate) fn try_attach_comment_assignment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let right_owner = owners.right;
    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_line = facts.comment_is_line;
    let comment_is_star = facts.comment_is_star;
    let token_before_is_assign = facts.token_before_is(TokenType::Assign);
    let token_after_span = context.token_after_span;
    let token_before_span = context.token_before_span;

    // inline block comments between assignment and rhs should stay inline with the rhs expression
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_before_is_assign
        && let Some(target_node) =
            right_owner.or_else(|| resolve_comment_seam_owner(context, seam_owner_cache))
    {
        let target_node = promote_rhs_expression_owner(
            tree,
            parents,
            target_node,
            token_after_span.map(|token| token.span),
        );
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments between assignment and rhs should bind to the rhs seam
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_assign
        && let Some(target_node) =
            right_owner.or_else(|| resolve_comment_seam_owner(context, seam_owner_cache))
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
            right_owner.or_else(|| resolve_comment_seam_owner(context, seam_owner_cache))
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

/// Resolve expression operator seam comment rules.
pub(crate) fn try_attach_comment_expression_operator(
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
    let token_before_is_open_parenthesis = facts.token_before_is(TokenType::OpenParenthesis);
    let token_before_is_assign = facts.token_before_is(TokenType::Assign);
    let token_before_is_colon = facts.token_before_is(TokenType::Colon);

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
    let starts_leading_type_grouping_operator =
        token_before_is_open_parenthesis || token_before_is_assign || token_before_is_colon;

    // leading type-grouping operator comments belong to the rhs type expression
    if token_after_is_elementwise_operator
        && starts_leading_type_grouping_operator
        && (comment_is_star || comment_is_line)
        && let Some(target_node) = context
            .token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
            .or(seam_owner)
    {
        let target_node = context
            .token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        let target_node = descend_owner_through_transparent_expression_wrappers(tree, target_node);
        let target_node =
            promote_owner_to_elementwise_binary_expression_ancestor(tree, parents, target_node)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if has_leading_newline {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

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
        && tree.get_node_type(target_node) == NodeType::Expression
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
        && tree.get_node_type(target_node) == NodeType::Expression
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

    // comments after `as` stay on cast seams, except mapped-type remap seams
    if token_before_is_as && !has_leading_newline && has_trailing_newline && comment_is_line {
        if let Some(target_node) = right_owner
            && tree.get_node_type(target_node) == NodeType::Expression
            && owner_has_type_mapped_expression_ancestor(tree, parents, target_node)
        {
            let target_expression = LocalNodeId::<Expression>::new(target_node);
            if let Expression::TypeTemplateLiteral { spans, .. } = tree.get(target_expression)
                && let Some(first_span) = spans.first()
            {
                let target_node = normalize_formatter_trivia_target_owner(tree, first_span.id);
                return Some((Some(target_node), AnnotationPosition::LinePostfix));
            }

            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        if let Some(target_node) = resolve_comment_seam_owner(context, seam_owner_cache) {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
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
        && let Some(right_target) = right_owner
    {
        let right_target = normalize_formatter_trivia_target_owner(tree, right_target);
        return Some((Some(right_target), AnnotationPosition::LinePrefix));
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
