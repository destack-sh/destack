use ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenType,
};
use destack_ast as ast;

use super::index::*;
use super::operator::resolve_comment_expression_operator_rules;
use super::owner::*;
use super::rule::normalize_owner_with_shared_end;
use super::seam::{
    CommentSeamContext, CommentSeamFacts, CommentSeamRuleState, resolve_comment_seam_owner,
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

/// Resolve expression and type seam comment rules.
pub(super) fn resolve_comment_expression_rules(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    state: &mut CommentSeamRuleState,
    left_owner: Option<u32>,
    right_owner: Option<u32>,
) -> Option<(Option<u32>, AnnotationPosition)> {
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
    let token_after_is_maybe = facts.token_after_is(TokenType::Maybe);
    let token_after_is_open_bracket = facts.token_after_is(TokenType::OpenBracket);
    let token_after_is_close_parenthesis = facts.token_after_is(TokenType::CloseParenthesis);
    let token_after_is_dot = facts.token_after_is(TokenType::Dot);
    let token_after_is_less_than = facts.token_after_is(TokenType::LessThan);
    let token_after_is_chain_or_index_boundary = token_after_is_dot || token_after_is_open_bracket;

    let token_before_is_open_parenthesis = facts.token_before_is(TokenType::OpenParenthesis);
    let token_before_is_open_brace = facts.token_before_is(TokenType::OpenBrace);
    let token_before_is_close_brace = facts.token_before_is(TokenType::CloseBrace);
    let token_before_is_semicolon = facts.token_before_is(TokenType::Semicolon);
    let token_before_is_spread = facts.token_before_is(TokenType::Spread);
    let token_before_is_logical_operator = matches!(
        facts.token_before_type,
        Some(TokenType::LogicalAnd | TokenType::LogicalOr | TokenType::Coalesce)
    );
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

    // comments between rest spread and binding names stay on the parameter owner
    if !has_leading_newline
        && !has_trailing_newline
        && token_before_is_spread
        && comment_is_star
        && let Some(target_node) = resolve_comment_seam_owner(context, state)
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

    // seam comments before chain and index operators stay with the left segment
    if !has_leading_newline
        && token_after_is_chain_or_index_boundary
        && !token_before_is_open_brace
        && let Some(target_node) = left_owner
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

    // comments between object open braces and computed keys stay inside the object
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_after_is_colon
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

    // ternary separator comments should stay inside the branch that follows `?`
    if comment_is_star
        && token_after_is_maybe
        && let Some(_ternary_owner) = shared_ternary_owner
    {
        if has_leading_newline {
            if let Some(target_node) = right_owner {
                let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
                return Some((Some(target_node), AnnotationPosition::LinePrefix));
            }
        } else if !has_trailing_newline && let Some(target_node) = left_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // ternary separator comments should stay inside the branch that follows `:`
    if comment_is_star
        && token_after_is_colon
        && let Some(_ternary_owner) = shared_ternary_owner
    {
        if has_leading_newline {
            if let Some(target_node) = right_owner {
                let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
                return Some((Some(target_node), AnnotationPosition::LinePrefix));
            }
        } else if !has_trailing_newline && let Some(target_node) = left_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // inline comments before `)` on grouped expressions should stay on the group
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_after_is_close_parenthesis
        && let Some(target_node) = left_owner
            .and_then(|owner| {
                promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
            })
            .or_else(|| {
                resolve_comment_seam_owner(context, state).and_then(|owner| {
                    promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
                })
            })
        && tree.get_node_type(target_node) == NodeType::Expression
        && matches!(
            tree.get(LocalNodeId::<Expression>::new(target_node)),
            Expression::Parenthesized { expression }
                if left_owner.is_some_and(|left_owner| left_owner != expression.id)
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
        && let Some(target_node) = resolve_comment_seam_owner(context, state)
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
            .or_else(|| resolve_comment_seam_owner(context, state))
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

    if let Some(decision) = resolve_comment_expression_operator_rules(
        tree,
        parents,
        context,
        facts,
        state,
        left_owner,
        right_owner,
    ) {
        return Some(decision);
    }

    None
}
