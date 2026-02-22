use ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenType,
};
use destack_ast as ast;

use crate::format::comments::index::FormatterTriviaOwnerIndex;
use crate::format::comments::operator::try_attach_comment_expression_operator;
use crate::format::comments::owner::{
    find_next_declaration_owner_from_token, find_preferred_owner_starting_at,
    find_smallest_owner_enclosing_range, find_smallest_owner_enclosing_token,
    lowest_common_owner_ancestor, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_by_shared_start,
    promote_owner_to_declaration_ancestor, promote_owner_to_node_type_ancestor,
    promote_owner_to_parenthesized_expression_ancestor,
};
use crate::format::comments::seam::{
    CommentAttachmentDecision, CommentAttachmentOwners, CommentSeamContext, CommentSeamFacts,
    CommentSeamOwnerCache, resolve_comment_seam_owner,
};

/// Promote cast owners to their consuming expression when grouped lhs wrappers are required.
fn promote_cast_owner_to_grouping_parenthesized_wrapper(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> u32 {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return owner_id;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    let Expression::TypeBinary {
        operator: ast::TypeBinaryOperator::Cast | ast::TypeBinaryOperator::Satisfies,
        ..
    } = tree.get(expression_id)
    else {
        return owner_id;
    };

    let Some(parent_id) = parents.get_by_id(owner_id) else {
        return owner_id;
    };
    if tree.get_node_type(parent_id) != NodeType::Expression {
        return owner_id;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = tree.get(parent_expression_id) else {
        return owner_id;
    };
    if expression.id != owner_id {
        return owner_id;
    }

    let Some(grandparent_id) = parents.get_by_id(parent_id) else {
        return owner_id;
    };
    if tree.get_node_type(grandparent_id) != NodeType::Expression {
        return owner_id;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    match tree.get(grandparent_expression_id) {
        Expression::If {
            kind: ast::IfKind::Ternary,
            ..
        } => parent_id,
        Expression::Binary { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Call { .. }
        | Expression::Must { .. }
        | Expression::Maybe { .. }
        | Expression::TaggedTemplateExpression { .. } => grandparent_id,
        _ => owner_id,
    }
}

/// Return the first dynamic argument owner for one call-like expression.
fn first_dynamic_argument_owner_for_call_like(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    match tree.get(expression_id) {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments.first().map(|argument_id| argument_id.id),
        _ => None,
    }
}

/// Promote one owner to a parent call/new expression when the owner is the call callee.
fn promote_owner_to_call_like_parent(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = owner_id;
    while let Some(parent_id) = parents.get_by_id(current_id) {
        if tree.get_node_type(parent_id) != NodeType::Expression {
            current_id = parent_id;
            continue;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        match tree.get(parent_expression_id) {
            Expression::Call { left, .. } | Expression::New { left, .. }
                if left.id == current_id =>
            {
                return Some(parent_id);
            }
            _ => {
                current_id = parent_id;
            }
        }
    }

    None
}

/// Promote one owner from a tree tag path to the enclosing tree expression when needed.
fn promote_owner_to_tree_expression_parent(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> u32 {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return owner_id;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    if !matches!(tree.get(expression_id), Expression::Path { .. }) {
        return owner_id;
    }

    let Some(parent_id) = parents.get_by_id(owner_id) else {
        return owner_id;
    };
    if tree.get_node_type(parent_id) != NodeType::Expression {
        return owner_id;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    if matches!(
        tree.get(parent_expression_id),
        Expression::TreeExpression { .. }
    ) {
        return parent_id;
    }

    owner_id
}

/// Promote one owner to the nearest labelled expression ancestor.
fn promote_owner_to_labelled_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if matches!(tree.get(expression_id), Expression::Labelled { .. }) {
                return Some(node_id);
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Resolve expression and type seam comment rules.
pub(crate) fn try_attach_comment_expression(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let left_owner = owners.left;
    let right_owner = owners.right;
    let token_after = context.token_after;
    let token_before_span = context.token_before_span;
    let token_after_span = context.token_after_span;

    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_line = facts.comment_is_line;
    let comment_is_star = facts.comment_is_star;

    let token_after_is_open_brace = facts.token_after_is(TokenType::OpenBrace);
    let token_after_is_open_parenthesis = facts.token_after_is(TokenType::OpenParenthesis);
    let token_after_is_colon = facts.token_after_is(TokenType::Colon);
    let token_after_is_open_bracket = facts.token_after_is(TokenType::OpenBracket);
    let token_after_is_close_brace = facts.token_after_is(TokenType::CloseBrace);
    let token_after_is_close_parenthesis = facts.token_after_is(TokenType::CloseParenthesis);
    let token_after_is_dot = facts.token_after_is(TokenType::Dot);
    let token_after_is_at = facts.token_after_is(TokenType::At);
    let token_after_is_less_than = facts.token_after_is(TokenType::LessThan);
    let token_after_is_chain_or_index_boundary = token_after_is_dot || token_after_is_open_bracket;

    let token_before_is_open_parenthesis = facts.token_before_is(TokenType::OpenParenthesis);
    let token_before_is_open_brace = facts.token_before_is(TokenType::OpenBrace);
    let token_before_is_comma = facts.token_before_is(TokenType::Comma);
    let token_before_is_maybe = facts.token_before_is(TokenType::Maybe);
    let token_before_is_colon = facts.token_before_is(TokenType::Colon);
    let token_before_is_close_brace = facts.token_before_is(TokenType::CloseBrace);
    let token_before_is_close_parenthesis = facts.token_before_is(TokenType::CloseParenthesis);
    let token_before_is_semicolon = facts.token_before_is(TokenType::Semicolon);
    let token_before_is_spread = facts.token_before_is(TokenType::Spread);
    let token_before_is_logical_operator = matches!(
        facts.token_before_type,
        Some(TokenType::LogicalAnd | TokenType::LogicalOr | TokenType::Coalesce)
    );
    let seam_has_shared_expression_owner = left_owner
        .zip(right_owner)
        .and_then(|(left_owner, right_owner)| {
            lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
        })
        .is_some_and(|owner| tree.get_node_type(owner) == NodeType::Expression);
    let shared_ternary_owner = left_owner
        .zip(right_owner)
        .and_then(|(left_owner, right_owner)| {
            lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
        })
        .and_then(|owner| {
            if tree.get_node_type(owner) != NodeType::Expression {
                return None;
            }

            let expression_id = LocalNodeId::<Expression>::new(owner);
            if matches!(
                tree.get(expression_id),
                Expression::If {
                    kind: ast::IfKind::Ternary,
                    ..
                }
            ) {
                return Some(owner);
            }

            None
        });
    let ternary_seam_owner = shared_ternary_owner.or_else(|| {
        let seam_owner = resolve_comment_seam_owner(context, seam_owner_cache)?;
        if tree.get_node_type(seam_owner) != NodeType::Expression {
            return None;
        }

        let seam_expression_id = LocalNodeId::<Expression>::new(seam_owner);
        if matches!(
            tree.get(seam_expression_id),
            Expression::If {
                kind: ast::IfKind::Ternary,
                ..
            }
        ) {
            return Some(seam_owner);
        }

        None
    });
    // decorator seams belong to declaration-specific routing.
    if token_after_is_at {
        return None;
    }

    // line comments after label colons stay with the labelled statement owner
    if !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && token_before_is_colon
        && let Some(target_node) = left_owner
            .and_then(|owner| promote_owner_to_labelled_expression_ancestor(tree, parents, owner))
            .or_else(|| {
                right_owner.and_then(|owner| {
                    promote_owner_to_labelled_expression_ancestor(tree, parents, owner)
                })
            })
    {
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // line comments after object member trailing commas inside call arguments stay on the member
    if comment_is_line
        && !has_leading_newline
        && token_before_is_comma
        && token_after_is_close_brace
        && left_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Argument)
    {
        let target_node = token_before_span.and_then(|comma_token| {
            let search_start = comma_token.span.start.saturating_sub(1);
            (search_start < comma_token.span.start).then(|| {
                find_smallest_owner_enclosing_range(tree, search_start, comma_token.span.start)
            })?
        });
        if let Some(target_node) = target_node.and_then(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Property)
        }) {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // trailing line comments after JSX child expression containers should stay on the child
    if comment_is_line
        && !has_leading_newline
        && token_before_is_close_brace
        && token_after_is_less_than
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Argument
        && tree.get_node_type(right_owner) == NodeType::Expression
        && matches!(
            tree.get(LocalNodeId::<Expression>::new(right_owner)),
            Expression::TreeExpression { .. }
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, left_owner);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // same-line comments after no-semi guards should stay on the guarded expression
    if token_before_is_semicolon
        && token_after_is_open_parenthesis
        && comment_is_star
        && let Some(mut target_node) = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
    {
        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if !matches!(tree.get(expression_id), Expression::Parenthesized { .. }) {
                target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            }
        } else {
            target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        }

        let position = if has_leading_newline {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

    // own-line comments between semicolon statements and array expressions stay on the rhs
    if has_leading_newline
        && comment_is_line
        && token_before_is_semicolon
        && token_after_is_open_bracket
        && let Some(target_node) = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
    {
        let target_node = token_after_span.map_or(target_node, |token| {
            promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
        });
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // own-line comments after `(` should bind to the full expression that starts at the rhs token
    if has_leading_newline
        && comment_is_line
        && token_before_is_open_parenthesis
        && let Some(token_after_span) = token_after_span
    {
        let target_node = find_smallest_owner_enclosing_token(tree, token_after_span.span)
            .or(right_owner)
            .map(|owner| {
                promote_owner_by_shared_start(tree, parents, owner, token_after_span.span.start)
            });
        if let Some(target_node) = target_node {
            let target_node =
                promote_cast_owner_to_grouping_parenthesized_wrapper(tree, parents, target_node);
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }
    }

    // inline comments after `{` before parenthesized expressions stay on the rhs expression
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_before_is_open_brace
        && token_after_is_open_bracket
    {
        let target_node = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
            .map(|owner| {
                token_after_span.map_or(owner, |token| {
                    promote_owner_by_shared_start(tree, parents, owner, token.span.start)
                })
            });
        if let Some(target_node) = target_node {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    // inline comments after `{` before parenthesized expressions stay on the rhs expression
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_before_is_open_brace
        && token_after_is_open_parenthesis
    {
        let target_node = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
            .map(|owner| {
                token_after_span.map_or(owner, |token| {
                    promote_owner_by_shared_start(tree, parents, owner, token.span.start)
                })
            });
        if let Some(target_node) = target_node {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    // comments between rest spread and binding names stay on the parameter owner
    if !has_leading_newline
        && !has_trailing_newline
        && token_before_is_spread
        && comment_is_star
        && let Some(target_node) = resolve_comment_seam_owner(context, seam_owner_cache)
            .or(left_owner)
            .or(right_owner)
    {
        let target_node =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Parameter)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // declaration generic head seams should stay on the declaration head
    if !has_leading_newline && has_trailing_newline && token_after_is_less_than {
        let declaration_target = token_after
            .and_then(|token_after_index| {
                find_next_declaration_owner_from_token(tree, owner_index, token_after_index)
            })
            .or_else(|| {
                right_owner.and_then(|owner| {
                    promote_owner_to_declaration_ancestor(tree, parents, owner).or_else(|| {
                        (tree.get_node_type(owner) == NodeType::Declaration).then_some(owner)
                    })
                })
            })
            .or_else(|| {
                left_owner.and_then(|owner| {
                    promote_owner_to_declaration_ancestor(tree, parents, owner).or_else(|| {
                        (tree.get_node_type(owner) == NodeType::Declaration).then_some(owner)
                    })
                })
            });

        if let Some(target_node) = declaration_target {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    // own-line seam comments before chain and index operators stay on the seam operation
    if has_leading_newline
        && token_after_is_chain_or_index_boundary
        && seam_has_shared_expression_owner
        && !token_before_is_comma
        && !token_before_is_open_brace
    {
        let seam_owner = resolve_comment_seam_owner(context, seam_owner_cache);

        if comment_is_line
            && let Some(target_node) = token_before_span
                .and_then(|token| find_preferred_owner_starting_at(tree, token.span))
                .or_else(|| {
                    token_before_span
                        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
                })
                .or(left_owner)
        {
            let target_node = normalize_owner_with_shared_end(
                tree,
                parents,
                target_node,
                token_before_span.map(|token| token.span),
            );
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        if let Some(target_node) = seam_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }
    }

    // seam comments before chain and index operators stay with the left segment
    if !has_leading_newline
        && token_after_is_chain_or_index_boundary
        && seam_has_shared_expression_owner
        && !token_before_is_comma
        && !token_before_is_open_brace
        && let Some(target_node) = token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(left_owner)
    {
        let target_node = normalize_owner_with_shared_end(
            tree,
            parents,
            target_node,
            token_before_span.map(|token| token.span),
        );
        if has_trailing_newline {
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // inline block comments right after `(` in call and new expressions stay on the first argument
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_before_is_open_parenthesis
        && let Some(seam_owner) = resolve_comment_seam_owner(context, seam_owner_cache)
        && let Some(target_node) = first_dynamic_argument_owner_for_call_like(tree, seam_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // inline block comments between call callees and `(` stay inside non-empty argument lists
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_after_is_open_parenthesis
        && let Some(seam_owner) = resolve_comment_seam_owner(context, seam_owner_cache)
        && tree.get_node_type(seam_owner) == NodeType::Expression
        && matches!(
            tree.get(LocalNodeId::<Expression>::new(seam_owner)),
            Expression::Call { .. } | Expression::New { .. }
        )
    {
        if let Some(target_node) = first_dynamic_argument_owner_for_call_like(tree, seam_owner) {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        if let Some(target_node) = token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(left_owner)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfix));
        }
    }

    // comments before parenthesized call-style cast targets should bind to the call expression
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_after_is_open_parenthesis
        && left_owner.is_some_and(|owner| tree.get_node_type(owner) != NodeType::Expression)
        && let Some(target_node) =
            right_owner.and_then(|owner| promote_owner_to_call_like_parent(tree, parents, owner))
    {
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments between object open braces and computed keys stay inside the object
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_before_is_open_brace
        && token_after_is_open_bracket
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // line comments after logical operators belong to the right operand
    if !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && token_before_is_logical_operator
        && let Some(target_node) = right_owner
    {
        let target_node = token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // trailing line comments after non-block if consequents stay on the consequent statement
    if !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && !token_before_is_close_parenthesis
        && let Some(seam_owner) = resolve_comment_seam_owner(context, seam_owner_cache)
        && tree.get_node_type(seam_owner) == NodeType::Expression
    {
        let seam_expression = LocalNodeId::<Expression>::new(seam_owner);
        if let Expression::If {
            kind: ast::IfKind::If,
            then_expression,
            else_expression,
            ..
        } = tree.get(seam_expression)
            && else_expression.is_none()
        {
            return Some((
                Some(then_expression.id),
                AnnotationPosition::LinePostfixBoundary,
            ));
        }
    }

    // line comments after ternary `:` stay with the consequent branch boundary
    if !has_leading_newline
        && comment_is_line
        && token_before_is_colon
        && !facts.token_before_is_return_type_colon
        && ternary_seam_owner.is_some()
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_owner_with_shared_end(
            tree,
            parents,
            target_node,
            token_before_span.map(|token| token.span),
        );
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // line comments before tree-expression container `}` stay on the container expression
    if !has_leading_newline
        && comment_is_line
        && token_after_is_close_brace
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

    // comments immediately before `:` should stay with the branch expression shape:
    // tree expressions continue on the alternate side, scalar expressions stay on the consequent
    if comment_is_star
        && token_after_is_colon
        && let Some(_ternary_owner) = ternary_seam_owner
        && let Some(left_target_node) = token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(left_owner)
    {
        if has_leading_newline && let Some(target_node) = right_owner {
            let target_node = promote_owner_to_tree_expression_parent(tree, parents, target_node);
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        let left_parenthesized_owner =
            promote_owner_to_parenthesized_expression_ancestor(tree, parents, left_target_node);
        let left_tree_candidate =
            promote_owner_to_tree_expression_parent(tree, parents, left_target_node);
        let left_is_tree_expression = tree.get_node_type(left_tree_candidate)
            == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(left_tree_candidate)),
                Expression::TreeExpression { .. }
            );

        if left_is_tree_expression
            && has_trailing_newline
            && let Some(target_node) = left_parenthesized_owner
        {
            let target_node = if tree.get_node_type(target_node) == NodeType::Expression {
                let parenthesized_id = LocalNodeId::<Expression>::new(target_node);
                match tree.get(parenthesized_id) {
                    Expression::Parenthesized { expression } => expression.id,
                    _ => target_node,
                }
            } else {
                target_node
            };
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        if left_is_tree_expression && let Some(target_node) = right_owner {
            let target_node = promote_owner_to_tree_expression_parent(tree, parents, target_node);
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        if tree.get_node_type(left_target_node) == NodeType::Expression {
            let left_expression_id = LocalNodeId::<Expression>::new(left_target_node);
            if let Expression::Parenthesized { expression } = tree.get(left_expression_id) {
                let target_node = normalize_formatter_trivia_target_owner(tree, expression.id);
                return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
            }
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, left_target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // comments immediately after `?` should stay inside the consequent branch
    if comment_is_star
        && token_before_is_maybe
        && let Some(_ternary_owner) = ternary_seam_owner
        && let Some(target_node) = right_owner
    {
        let target_node = promote_owner_to_tree_expression_parent(tree, parents, target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments immediately after `:` should stay inside the alternate branch
    if comment_is_star
        && token_before_is_colon
        && let Some(_ternary_owner) = ternary_seam_owner
        && let Some(target_node) = right_owner
    {
        let target_node = promote_owner_to_tree_expression_parent(tree, parents, target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // inline comments before `)` in ternary branches stay on the inner branch expression
    if !has_leading_newline
        && comment_is_star
        && token_after_is_close_parenthesis
        && ternary_seam_owner.is_some()
        && let Some(target_node) = token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(left_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // inline comments before `)` on grouped expressions should stay on the group
    if !has_leading_newline
        && comment_is_star
        && token_after_is_close_parenthesis
        && let Some(target_node) = token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(left_owner)
            .and_then(|owner| {
                promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
            })
            .or_else(|| {
                resolve_comment_seam_owner(context, seam_owner_cache).and_then(|owner| {
                    promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
                })
            })
        && tree.get_node_type(target_node) == NodeType::Expression
        && matches!(
            tree.get(LocalNodeId::<Expression>::new(target_node)),
            Expression::Parenthesized { .. }
        )
    {
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // comments between object open braces and computed keys stay inside the object
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_before_is_open_brace
        && token_after_is_open_bracket
        && let Some(target_node) = resolve_comment_seam_owner(context, seam_owner_cache)
    {
        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if let Expression::ObjectExpression { properties, .. } = tree.get(expression_id)
                && let Some(first_property) = properties.first().copied()
            {
                return Some((Some(first_property.id), AnnotationPosition::LinePrefix));
            }
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments before closure-cast object literals stay with the rhs cast target
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_after_is_open_brace
        && token_before_is_open_parenthesis
        && let Some(mut target_node) = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
            .or_else(|| resolve_comment_seam_owner(context, seam_owner_cache))
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

        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if !matches!(tree.get(expression_id), Expression::Parenthesized { .. }) {
                target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            }
        } else {
            target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        }

        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments before parenthesized cast targets stay with the grouped expression
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_after_is_open_parenthesis
        && let Some(target_node) = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
            .or_else(|| resolve_comment_seam_owner(context, seam_owner_cache))
            .map(|owner| {
                token_after_span.map_or(owner, |token| {
                    promote_owner_by_shared_start(tree, parents, owner, token.span.start)
                })
            })
            .and_then(|owner| {
                promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
                    .or(Some(owner))
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let target_node = if left_owner
            .is_some_and(|owner| tree.get_node_type(owner) == NodeType::Declaration)
            && tree.get_node_type(target_node) == NodeType::Expression
            && let Expression::Parenthesized { expression } =
                tree.get(LocalNodeId::<Expression>::new(target_node))
        {
            expression.id
        } else {
            target_node
        };
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    if let Some(decision) = try_attach_comment_expression_operator(
        tree,
        parents,
        context,
        facts,
        seam_owner_cache,
        owners,
    ) {
        return Some(decision);
    }

    None
}
