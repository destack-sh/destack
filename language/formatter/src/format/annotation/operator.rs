use ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenType,
};
use destack_ast as ast;

use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner, previous_non_newline_token_index,
};
use super::facts::token_type_is_comment_trivia;
use super::ownership::{
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    promote_owner_by_shared_start, promote_owner_to_node_type_ancestor,
    promote_owner_to_satisfies_expression_ancestor, promote_rhs_expression_owner,
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

/// Return whether one owner is one cast-like type-binary expression.
fn owner_is_cast_or_satisfies_type_binary_expression(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::TypeBinary {
            operator: ast::TypeBinaryOperator::Cast | ast::TypeBinaryOperator::Satisfies,
            ..
        }
    )
}

/// Return assignment token index for one seam when comments follow `=`.
fn assignment_token_index_for_seam(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> Option<usize> {
    if seam.token_before_is(TokenType::Assign) {
        return context.token_before;
    }

    let token_before_index = context.token_before?;
    let token_before_type = context.semantic_tokens[token_before_index].token.ty;
    if !token_type_is_comment_trivia(token_before_type) {
        return None;
    }

    let previous_index =
        previous_non_newline_token_index(context.semantic_tokens, token_before_index)?;
    let previous_type = context.semantic_tokens[previous_index].token.ty;
    (previous_type == TokenType::Assign).then_some(previous_index)
}

/// Return rhs owner for one assignment seam.
fn assignment_rhs_owner_for_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
) -> Option<u32> {
    let assignment_owner = assignment_token_index_for_seam(context, seam)
        .and_then(|token_index| {
            let token_span = context.semantic_tokens[token_index].span;
            find_smallest_owner_enclosing_token(tree, token_span)
        })
        .and_then(|owner_id| {
            if tree.get_node_type(owner_id) == NodeType::Expression {
                Some(owner_id)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
            }
        })
        .and_then(|expression_owner| {
            let expression_id = LocalNodeId::<Expression>::new(expression_owner);
            match tree.get(expression_id) {
                Expression::Assign { right, .. } => Some(right.id),
                _ => None,
            }
        });
    let target_node = assignment_owner.or_else(|| {
        following_owner.or_else(|| comment_enclosing_owner(context, enclosing_owner_cache))
    })?;

    Some(promote_rhs_expression_owner(
        tree,
        parents,
        target_node,
        context.token_after_span.map(|token| token.span),
    ))
}

/// Resolve assignment seam comment rules.
pub(crate) fn try_attach_comment_assignment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let following_owner = owners.following;
    let has_leading_newline = seam.has_leading_newline;
    let has_trailing_newline = seam.has_trailing_newline;
    let comment_is_line = seam.comment_is_line;
    let comment_is_star = seam.comment_is_star;
    let seam_follows_assignment = assignment_token_index_for_seam(context, seam).is_some();
    let token_before_span = context.token_before_span;

    // inline block comments between assignment and rhs should stay inline with the rhs expression
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && seam_follows_assignment
        && let Some(target_node) = assignment_rhs_owner_for_seam(
            tree,
            parents,
            context,
            seam,
            following_owner,
            enclosing_owner_cache,
        )
    {
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments between assignment and rhs should bind to the rhs seam
    if !has_leading_newline
        && has_trailing_newline
        && seam_follows_assignment
        && let Some(target_node) = assignment_rhs_owner_for_seam(
            tree,
            parents,
            context,
            seam,
            following_owner,
            enclosing_owner_cache,
        )
    {
        let comment_starts_on_assign_line = token_before_span.is_some_and(|before_token| {
            context
                .file
                .is_same_line(before_token.span.start, context.trivia.span.start)
        });

        if comment_is_line && comment_starts_on_assign_line {
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // own-line comments between assignment and rhs stay on the rhs value region
    if has_leading_newline
        && seam_follows_assignment
        && let Some(target_node) = assignment_rhs_owner_for_seam(
            tree,
            parents,
            context,
            seam,
            following_owner,
            enclosing_owner_cache,
        )
    {
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}

/// Resolve expression operator seam comment rules.
pub(crate) fn try_attach_comment_expression_operator(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;
    let has_leading_newline = seam.has_leading_newline;
    let has_trailing_newline = seam.has_trailing_newline;
    let comment_is_line = seam.comment_is_line;
    let comment_is_star = seam.comment_is_star;
    let comment_is_multiline_star = seam.comment_is_multiline_star;

    let token_after_is_maybe = seam.token_after_is(TokenType::Maybe);
    let token_after_is_as = seam.token_after_is_keyword(CommentSeamKeyword::As);
    let token_after_is_satisfies = seam.token_after_is_keyword(CommentSeamKeyword::Satisfies);
    let token_after_is_const = seam.token_after_is_keyword(CommentSeamKeyword::Const);
    let token_after_is_elementwise_operator = matches!(
        seam.token_after_type,
        Some(TokenType::ElementwiseAnd | TokenType::ElementwiseOr | TokenType::ElementwiseXor)
    );
    let token_before_is_open_parenthesis = seam.token_before_is(TokenType::OpenParenthesis);
    let token_before_is_assign = seam.token_before_is(TokenType::Assign);
    let token_before_is_colon = seam.token_before_is(TokenType::Colon);

    let token_before_is_as = seam.token_before_is_keyword(CommentSeamKeyword::As);
    let token_before_is_satisfies = seam.token_before_is_keyword(CommentSeamKeyword::Satisfies);
    let token_before_is_less_than = seam.token_before_is(TokenType::LessThan);
    let token_before_is_elementwise_operator = matches!(
        seam.token_before_type,
        Some(TokenType::ElementwiseAnd | TokenType::ElementwiseOr | TokenType::ElementwiseXor)
    );
    let token_before_is_elementwise_or =
        matches!(seam.token_before_type, Some(TokenType::ElementwiseOr));
    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache);
    let seam_is_cast_or_satisfies_operator_context =
        [enclosing_owner, preceding_owner, following_owner]
            .into_iter()
            .flatten()
            .any(|owner_id| {
                let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
                    Some(owner_id)
                } else {
                    promote_owner_to_node_type_ancestor(
                        tree,
                        parents,
                        owner_id,
                        NodeType::Expression,
                    )
                };

                expression_owner.is_some_and(|expression_owner| {
                    owner_is_cast_or_satisfies_type_binary_expression(tree, expression_owner)
                })
            });
    let cast_or_satisfies_rhs_owner = [enclosing_owner, preceding_owner, following_owner]
        .into_iter()
        .flatten()
        .find_map(|owner_id| {
            let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
                Some(owner_id)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
            }?;

            let expression_id = LocalNodeId::<Expression>::new(expression_owner);
            match tree.get(expression_id) {
                Expression::TypeBinary {
                    operator: ast::TypeBinaryOperator::Cast | ast::TypeBinaryOperator::Satisfies,
                    right,
                    ..
                } => Some(right.id),
                Expression::TypeUnary {
                    operator: ast::TypeUnaryOperator::AsConst,
                    right,
                } => Some(right.id),
                _ => None,
            }
        });
    let as_const_type_unary_owner = [enclosing_owner, preceding_owner, following_owner]
        .into_iter()
        .flatten()
        .find_map(|owner_id| {
            let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
                Some(owner_id)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
            }?;

            let expression_id = LocalNodeId::<Expression>::new(expression_owner);
            match tree.get(expression_id) {
                Expression::TypeUnary {
                    operator: ast::TypeUnaryOperator::AsConst,
                    ..
                } => Some(expression_owner),
                _ => None,
            }
        });
    let starts_leading_type_grouping_operator = token_before_is_open_parenthesis
        || token_before_is_assign
        || token_before_is_colon
        || preceding_owner.is_none();

    // leading type-grouping operator comments belong to the rhs type expression
    if token_after_is_elementwise_operator
        && starts_leading_type_grouping_operator
        && (comment_is_star || comment_is_line)
        && let Some(target_node) = context
            .token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(following_owner)
            .or(enclosing_owner)
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
        let position = if has_trailing_newline || comment_is_multiline_star {
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
        && let Some(target_node) = preceding_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // optional call line comments should stay on the full optional expression
    if !has_leading_newline
        && has_trailing_newline
        && token_after_is_maybe
        && comment_is_line
        && let Some(target_node) = comment_enclosing_owner(context, enclosing_owner_cache)
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
        && let Some(target_node) = preceding_owner
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
        && let Some(target_node) = preceding_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // comments after `as` stay on cast seams, except mapped-type remap seams
    if (token_before_is_as || token_before_is_satisfies)
        && comment_is_multiline_star
        && (has_leading_newline || has_trailing_newline)
        && seam_is_cast_or_satisfies_operator_context
        && let Some(target_node) = following_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // line comments after `as` and `satisfies` stay on rhs prefixes
    if (token_before_is_as || token_before_is_satisfies)
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && seam_is_cast_or_satisfies_operator_context
        && !token_after_is_const
        && let Some(target_node) = cast_or_satisfies_rhs_owner.or(following_owner).or_else(|| {
            context
                .token_after_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        })
    {
        let target_node = context
            .token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // line comments between `as` and `const` stay as infix seams on `as const` unary expressions
    if token_before_is_as
        && token_after_is_const
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && let Some(target_node) = as_const_type_unary_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockInfix));
    }

    // line comments after `<` in satisfies rhs type arguments stay with the rhs type
    if token_before_is_less_than
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && comment_enclosing_owner(context, enclosing_owner_cache)
            .or(preceding_owner)
            .and_then(|target_node| {
                promote_owner_to_satisfies_expression_ancestor(tree, parents, target_node)
            })
            .is_some()
        && let Some(right_target) = following_owner
    {
        let right_target = normalize_formatter_trivia_target_owner(tree, right_target);
        return Some((Some(right_target), AnnotationPosition::LinePrefix));
    }

    // line comments after type and bitwise operators should stay with the rhs operand
    if token_before_is_elementwise_operator
        && !has_trailing_newline
        && comment_is_star
        && preceding_owner.is_none()
        && let Some(target_node) = following_owner
    {
        let target_node = context
            .token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // line comments after type and bitwise operators should stay with the rhs operand
    if token_before_is_elementwise_operator
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
    {
        if token_before_is_elementwise_or
            && let Some(target_node) = preceding_owner.or(enclosing_owner)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        let Some(target_node) = following_owner else {
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
        && let Some(target_node) = comment_enclosing_owner(context, enclosing_owner_cache)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}
