use ast::{
    AnnotationPosition, Declaration, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenType,
};
use destack_ast as ast;

use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner,
};
use super::facts::{previous_non_trivia_token_index, token_type_is_trivia};
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

/// Descend one owner to the terminal right operand of one elementwise binary chain.
fn descend_owner_to_terminal_elementwise_operand(tree: &NodeTree, owner_id: u32) -> u32 {
    let mut current_id = owner_id;

    loop {
        if tree.get_node_type(current_id) != NodeType::Expression {
            return current_id;
        }

        let expression_id = LocalNodeId::<Expression>::new(current_id);
        let Expression::Binary {
            operator:
                ast::BinaryOperator::ElementwiseAnd
                | ast::BinaryOperator::ElementwiseOr
                | ast::BinaryOperator::ElementwiseXor,
            right,
            ..
        } = tree.get(expression_id)
        else {
            return current_id;
        };

        current_id = right.id;
    }
}

/// Return whether comment trivia appears between one seam comment and its next non-trivia token.
fn seam_has_intervening_comment_before_next_token(context: &CommentSeamContext<'_>) -> bool {
    let Some(token_after_span) = context.token_after_span else {
        return false;
    };
    let comment_trivia = context.tree.comment_trivia();
    let mut trivia_index =
        comment_trivia.partition_point(|trivia| trivia.span.end <= context.trivia.span.end);
    while let Some(trivia) = comment_trivia.get(trivia_index).copied() {
        if trivia.span.start >= token_after_span.span.start {
            break;
        }

        if trivia.comment != context.trivia.comment {
            return true;
        }
        trivia_index += 1;
    }

    false
}

/// Return whether two owners are in the same elementwise binary chain.
fn owners_share_elementwise_binary_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    left_owner_id: u32,
    right_owner_id: u32,
) -> bool {
    let left_binary_owner =
        promote_owner_to_elementwise_binary_expression_ancestor(tree, parents, left_owner_id);
    let right_binary_owner =
        promote_owner_to_elementwise_binary_expression_ancestor(tree, parents, right_owner_id);

    left_binary_owner.is_some() && left_binary_owner == right_binary_owner
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

/// Promote one owner to the nearest cast or satisfies type-binary expression ancestor.
fn promote_owner_to_cast_or_satisfies_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if owner_is_cast_or_satisfies_type_binary_expression(tree, node_id) {
            return Some(node_id);
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Return whether one rhs owner is one single-segment path with multiple static arguments.
fn rhs_is_single_segment_multi_static_argument_path(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::Path {
            path,
            static_arguments: Some(static_arguments),
        } if path.segments.len() == 1 && static_arguments.len() > 1
    )
}

/// Promote one owner to the mapped-type key-remap path owner.
fn promote_owner_to_mapped_type_key_remap_path_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if let Expression::TypeTemplateLiteral { spans, .. } = tree.get(expression_id)
                && let Some(parent_id) = parents.get_by_id(node_id)
                && tree.get_node_type(parent_id) == NodeType::Expression
            {
                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                if let Expression::TypeMapped { parameter, .. } = tree.get(parent_expression_id)
                    && parameter.key_remap == Some(LocalNodeId::new(node_id))
                    && let Some(first_span) = spans.first().copied()
                    && matches!(
                        tree.get(first_span),
                        Expression::Path {
                            static_arguments: Some(_),
                            ..
                        }
                    )
                {
                    return Some(first_span.id);
                }
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Promote one owner to the nearest mapped-type value owner.
fn promote_owner_to_mapped_type_value_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if let Expression::TypeMapped { value, .. } = tree.get(expression_id) {
                return Some(value.id);
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
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
    if !token_type_is_trivia(token_before_type) {
        return None;
    }

    let previous_index =
        previous_non_trivia_token_index(context.semantic_tokens, token_before_index)?;
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
    let assignment_owner = assignment_token_index_for_seam(context, seam).and_then(|token_index| {
        let token_span = context.semantic_tokens[token_index].span;
        let owner_id = find_smallest_owner_enclosing_token(tree, token_span)?;

        let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
            Some(owner_id)
        } else {
            promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
        };
        if let Some(expression_owner) = expression_owner {
            let expression_id = LocalNodeId::<Expression>::new(expression_owner);
            if let Expression::Assign { right, .. } = tree.get(expression_id) {
                return Some(right.id);
            }
        }

        let declaration_owner = if tree.get_node_type(owner_id) == NodeType::Declaration {
            Some(owner_id)
        } else {
            promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Declaration)
        }?;
        let declaration_id = LocalNodeId::<Declaration>::new(declaration_owner);
        match tree.get(declaration_id) {
            Declaration::Type { value, .. } => Some(value.id),
            _ => None,
        }
    });
    let target_node = assignment_owner
        .or_else(|| {
            context
                .token_after_span
                .and_then(|token_after| find_smallest_owner_enclosing_token(tree, token_after.span))
        })
        .or_else(|| {
            following_owner.or_else(|| comment_enclosing_owner(context, enclosing_owner_cache))
        })?;

    let mut target_node = promote_rhs_expression_owner(
        tree,
        parents,
        target_node,
        context.token_after_span.map(|token| token.span),
    );

    if tree.get_node_type(target_node) == NodeType::Declaration {
        let declaration_id = LocalNodeId::<Declaration>::new(target_node);
        if let Declaration::Type { value, .. } = tree.get(declaration_id) {
            target_node = value.id;
        }
    }

    Some(target_node)
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
    let assignment_token_index = assignment_token_index_for_seam(context, seam);
    let seam_follows_assignment = assignment_token_index.is_some();
    let comment_starts_on_assignment_line = assignment_token_index
        .and_then(|index| context.semantic_tokens.get(index).copied())
        .is_some_and(|token| {
            context
                .file
                .is_same_line(token.span.start, context.trivia.span.start)
        });

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
        let position = if comment_is_line && comment_starts_on_assignment_line {
            AnnotationPosition::LinePrefix
        } else {
            AnnotationPosition::BlockPrefix
        };
        return Some((Some(target_node), position));
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
    let token_before_is_elementwise_and = seam.token_before_is(TokenType::ElementwiseAnd);
    let token_before_is_elementwise_or = seam.token_before_is(TokenType::ElementwiseOr);
    let token_before_is_elementwise_xor = seam.token_before_is(TokenType::ElementwiseXor);
    let token_after_is_elementwise_and = seam.token_after_is(TokenType::ElementwiseAnd);
    let token_after_is_elementwise_or = seam.token_after_is(TokenType::ElementwiseOr);
    let token_after_is_elementwise_xor = seam.token_after_is(TokenType::ElementwiseXor);
    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache);
    let seam_is_cast_or_satisfies_operator_context =
        [enclosing_owner, preceding_owner, following_owner]
            .into_iter()
            .flatten()
            .any(|owner_id| {
                promote_owner_to_cast_or_satisfies_expression_ancestor(tree, parents, owner_id)
                    .is_some()
            });
    let cast_or_satisfies_expression_owner = [enclosing_owner, preceding_owner, following_owner]
        .into_iter()
        .flatten()
        .find_map(|owner_id| {
            promote_owner_to_cast_or_satisfies_expression_ancestor(tree, parents, owner_id)
        });
    let cast_or_satisfies_rhs_owner = cast_or_satisfies_expression_owner.and_then(|owner_id| {
        let expression_id = LocalNodeId::<Expression>::new(owner_id);
        match tree.get(expression_id) {
            Expression::TypeBinary { right, .. } => Some(right.id),
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
        || token_before_is_as
        || token_before_is_satisfies
        || preceding_owner.is_none();

    // assignment seams that lead into type-grouping operators stay on the rhs value region
    if token_after_is_elementwise_operator
        && token_before_is_assign
        && (comment_is_star || comment_is_line)
        && let Some(target_node) = if token_after_is_elementwise_and {
            context
                .token_after_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
                .or(following_owner)
                .or_else(|| {
                    assignment_rhs_owner_for_seam(
                        tree,
                        parents,
                        context,
                        seam,
                        following_owner,
                        enclosing_owner_cache,
                    )
                })
        } else {
            assignment_rhs_owner_for_seam(
                tree,
                parents,
                context,
                seam,
                following_owner,
                enclosing_owner_cache,
            )
            .or(following_owner)
            .or(enclosing_owner)
        }
    {
        let target_node = if token_after_is_elementwise_or || token_after_is_elementwise_xor {
            promote_owner_to_elementwise_binary_expression_ancestor(tree, parents, target_node)
                .unwrap_or(target_node)
        } else {
            target_node
        };
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if has_trailing_newline || comment_is_multiline_star {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

    // leading type-grouping operator comments belong to the rhs type expression
    if token_after_is_elementwise_operator
        && starts_leading_type_grouping_operator
        && (comment_is_star || comment_is_line)
        && let Some(target_node) = following_owner
            .or_else(|| {
                context
                    .token_after_span
                    .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            })
            .or(enclosing_owner)
    {
        let has_intervening_comment_before_next_token =
            seam_has_intervening_comment_before_next_token(context);
        let target_node = context
            .token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        let target_node = descend_owner_through_transparent_expression_wrappers(tree, target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if comment_is_multiline_star
            || (has_trailing_newline
                && (comment_is_line || has_intervening_comment_before_next_token))
        {
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

    // line comments before type and bitwise separators stay with the left operand
    if has_trailing_newline
        && comment_is_line
        && token_after_is_elementwise_operator
        && let Some(target_node) = preceding_owner
    {
        let target_node = descend_owner_to_terminal_elementwise_operand(tree, target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if has_leading_newline {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::LinePostfix
        };
        return Some((Some(target_node), position));
    }

    // line comments after mapped-type value `:` should stay on the mapped value boundary
    if token_before_is_colon
        && !seam.token_before_is_return_type_colon
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && let Some(target_node) = [following_owner, preceding_owner, enclosing_owner]
            .into_iter()
            .flatten()
            .find_map(|owner_id| promote_owner_to_mapped_type_value_owner(tree, parents, owner_id))
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
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

    // own-line comments inside cast and satisfies seams attach to the rhs type
    if has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && seam_is_cast_or_satisfies_operator_context
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
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // mapped-type key remap seams after `as` should stay on the remap path owner
    if token_before_is_as
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && let Some(target_node) = [following_owner, enclosing_owner, preceding_owner]
            .into_iter()
            .flatten()
            .find_map(|owner_id| {
                promote_owner_to_mapped_type_key_remap_path_owner(tree, parents, owner_id)
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // line comments after `as` and `satisfies` should follow operator ownership:
    // - preserve rhs-prefix seam only for single-segment satisfies type argument lists
    // - otherwise keep the comment on the full cast/satisfies expression boundary
    if (token_before_is_as || token_before_is_satisfies)
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
    {
        let keeps_rhs_prefix = token_before_is_satisfies
            && cast_or_satisfies_rhs_owner.is_some_and(|owner_id| {
                rhs_is_single_segment_multi_static_argument_path(tree, owner_id)
            });

        if keeps_rhs_prefix
            && let Some(target_node) =
                cast_or_satisfies_rhs_owner.or(following_owner).or_else(|| {
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

        if token_before_is_as
            && token_after_is_const
            && let Some(target_node) = as_const_type_unary_owner
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfix));
        }

        if seam_is_cast_or_satisfies_operator_context
            && let Some(target_node) = cast_or_satisfies_expression_owner
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfix));
        }
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
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // multiline or own-line block comments after type and bitwise operators stay with the rhs operand
    if token_before_is_elementwise_operator
        && has_trailing_newline
        && comment_is_star
        && let Some(target_node) = following_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if has_leading_newline || comment_is_multiline_star {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

    // line comments after type and bitwise operators should stay on the left operand boundary
    if token_before_is_elementwise_operator
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
    {
        if token_before_is_elementwise_and
            && let Some(target_node) = context
                .token_after_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
                .or(following_owner)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }

        if (token_before_is_elementwise_or || token_before_is_elementwise_xor)
            && let Some(preceding_node) = preceding_owner
            && let Some(following_node) = following_owner
            && owners_share_elementwise_binary_expression_ancestor(
                tree,
                parents,
                preceding_node,
                following_node,
            )
        {
            let target_node = descend_owner_to_terminal_elementwise_operand(tree, preceding_node);
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        let Some(target_node) = following_owner else {
            return None;
        };
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
