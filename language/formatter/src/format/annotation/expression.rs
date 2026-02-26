use ast::{
    AnnotationPosition, Declaration, Expression, FunctionMode, LocalNodeId, NodeParentIndex,
    NodeTree, NodeType, TokenType,
};
use destack_ast as ast;

use super::attachment::{
    FormatterTriviaOwnerIndex, attach_line_comment_after_ternary_colon,
    attach_star_comment_before_ternary_colon,
    attach_trailing_comma_close_brace_property_line_comment, if_expression_then_owner_without_else,
    promote_owner_to_tree_expression_parent,
};
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, comment_enclosing_owner, seam_is_template_interpolation_open_brace,
};
use super::operator::try_attach_comment_expression_operator;
use super::ownership::{
    find_owner_at_or_after_token, find_preferred_owner_starting_at,
    find_smallest_owner_enclosing_token, lowest_common_owner_ancestor,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_start, promote_owner_to_declaration_ancestor,
    promote_owner_to_node_type_ancestor, promote_owner_to_parenthesized_expression_ancestor,
};
use super::semicolon::attach_semicolon_guard_own_line_comment;

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

/// Return whether one owner belongs to a `new (...) => ...` declaration signature.
fn owner_is_new_signature_declaration(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    let Some(declaration_owner) = promote_owner_to_declaration_ancestor(tree, parents, owner_id)
    else {
        return false;
    };

    if tree.get_node_type(declaration_owner) != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(declaration_owner);
    let Declaration::Function { signature, .. } = tree.get(declaration_id) else {
        return false;
    };

    signature.mode == Some(FunctionMode::New)
}

/// Return whether one owner is a ternary if expression.
fn owner_is_ternary_if_expression(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::If {
            kind: ast::IfKind::Ternary,
            ..
        }
    )
}

/// Return whether one owner is a call or new expression.
fn owner_is_call_or_new_expression(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. }
    )
}

/// Return whether one owner belongs to a do-while expression ancestry.
fn owner_has_do_while_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    let mut current_owner = Some(owner_id);
    while let Some(node_id) = current_owner {
        if tree.get_node_type(node_id) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(node_id)),
                Expression::While {
                    kind: ast::WhileKind::DoWhile,
                    ..
                }
            )
        {
            return true;
        }

        current_owner = parents.get_by_id(node_id);
    }

    false
}

/// Promote one owner to the nearest tagged-template expression ancestor.
fn promote_owner_to_tagged_template_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_owner = Some(owner_id);
    while let Some(node_id) = current_owner {
        if tree.get_node_type(node_id) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(node_id)),
                Expression::TaggedTemplateExpression { .. }
            )
        {
            return Some(node_id);
        }

        current_owner = parents.get_by_id(node_id);
    }

    None
}

/// Resolve expression and type seam comment rules.
pub(crate) fn try_attach_comment_expression(
    tree: &NodeTree,
    _owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;
    let token_before_span = context.token_before_span;
    let token_after_span = context.token_after_span;
    let token_before_source_span = token_before_span.map(|token| token.span);
    let token_after_source_span = token_after_span.map(|token| token.span);
    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache);
    let preceding_token_owner =
        token_before_span.and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let following_token_owner =
        token_after_span.and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));

    let has_leading_newline = seam.has_leading_newline;
    let has_trailing_newline = seam.has_trailing_newline;
    let comment_is_line = seam.comment_is_line;
    let comment_is_star = seam.comment_is_star;
    let is_inline_star_comment = !has_leading_newline && !has_trailing_newline && comment_is_star;
    let is_trailing_line_comment = !has_leading_newline && has_trailing_newline && comment_is_line;

    let token_after_is_open_brace = seam.token_after_is(TokenType::OpenBrace);
    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);
    let token_after_is_colon = seam.token_after_is(TokenType::Colon);
    let token_after_is_spread = seam.token_after_is(TokenType::Spread);
    let token_after_is_open_bracket = seam.token_after_is(TokenType::OpenBracket);
    let token_after_is_close_brace = seam.token_after_is(TokenType::CloseBrace);
    let token_after_is_close_parenthesis = seam.token_after_is(TokenType::CloseParenthesis);
    let token_after_is_dot = seam.token_after_is(TokenType::Dot);
    let token_after_is_at = seam.token_after_is(TokenType::At);
    let token_after_is_less_than = seam.token_after_is(TokenType::LessThan);
    let token_after_is_chain_or_index_boundary = token_after_is_dot || token_after_is_open_bracket;
    let token_after_is_open_bracket_or_open_parenthesis =
        token_after_is_open_bracket || token_after_is_open_parenthesis;

    let token_before_is_open_parenthesis = seam.token_before_is(TokenType::OpenParenthesis);
    let token_before_is_open_brace = seam.token_before_is(TokenType::OpenBrace);
    let token_before_is_arrow =
        seam.token_before_is(TokenType::Arrow) || seam.token_before_is(TokenType::ArrowWide);
    let token_before_is_comma = seam.token_before_is(TokenType::Comma);
    let token_before_is_maybe = seam.token_before_is(TokenType::Maybe);
    let token_before_is_colon = seam.token_before_is(TokenType::Colon);
    let token_before_is_close_brace = seam.token_before_is(TokenType::CloseBrace);
    let token_before_is_close_parenthesis = seam.token_before_is(TokenType::CloseParenthesis);
    let token_before_is_semicolon = seam.token_before_is(TokenType::Semicolon);
    let token_before_is_spread = seam.token_before_is(TokenType::Spread);
    let token_before_is_logical_operator = matches!(
        seam.token_before_type,
        Some(TokenType::LogicalAnd | TokenType::LogicalOr | TokenType::Coalesce)
    );
    let seam_is_new_signature_declaration = preceding_owner
        .is_some_and(|owner| owner_is_new_signature_declaration(tree, parents, owner))
        || following_owner
            .is_some_and(|owner| owner_is_new_signature_declaration(tree, parents, owner));
    let shared_owner =
        preceding_owner
            .zip(following_owner)
            .and_then(|(preceding_owner, following_owner)| {
                lowest_common_owner_ancestor(tree, parents, preceding_owner, following_owner)
            });
    let seam_has_shared_expression_owner =
        shared_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Expression);
    let shared_ternary_owner =
        shared_owner.filter(|owner| owner_is_ternary_if_expression(tree, *owner));
    let ternary_enclosing_owner = shared_ternary_owner
        .or(enclosing_owner.filter(|owner| owner_is_ternary_if_expression(tree, *owner)));
    let is_ternary_seam = ternary_enclosing_owner.is_some();
    let ternary_following_owner = if is_ternary_seam {
        following_owner.map(|owner| {
            let owner = promote_owner_to_tree_expression_parent(tree, parents, owner);
            normalize_formatter_trivia_target_owner(tree, owner)
        })
    } else {
        None
    };
    let enclosing_owner_is_call_or_new =
        enclosing_owner.is_some_and(|owner| owner_is_call_or_new_expression(tree, owner));
    let enclosing_owner_first_dynamic_argument =
        enclosing_owner.and_then(|owner| first_dynamic_argument_owner_for_call_like(tree, owner));
    // decorator seams belong to declaration-specific routing.
    if token_after_is_at {
        return None;
    }

    // line comments right after `${` stay on the interpolation expression owner
    if comment_is_line
        && seam_is_template_interpolation_open_brace(context, seam)
        && let Some(target_node) = following_token_owner.or(following_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // inline block comments between a tag expression and template literal stay on the tagged seam
    if is_inline_star_comment
        && seam.token_after_is(TokenType::TemplateString)
        && let Some(target_node) = enclosing_owner
            .or(following_owner)
            .or(following_token_owner)
            .or(preceding_owner)
            .and_then(|owner| {
                promote_owner_to_tagged_template_expression_ancestor(tree, parents, owner)
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockInfix));
    }

    // comments after `=>` should stay with the lambda body expression
    if token_before_is_arrow
        && (comment_is_star || comment_is_line)
        && let Some(target_node) = following_token_owner
            .or(following_owner)
            .or(enclosing_owner)
            .map(|owner| {
                token_after_span.map_or(owner, |token| {
                    promote_owner_by_shared_start(tree, parents, owner, token.span.start)
                })
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if has_leading_newline || has_trailing_newline {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

    // own-line semicolon-guard seams resolve in expression routing for both semicolon-adjacent shapes
    if (seam.token_before_is(TokenType::Semicolon) || seam.token_after_is(TokenType::Semicolon))
        && let Some(attachment) =
            attach_semicolon_guard_own_line_comment(tree, parents, context, seam, owners)
    {
        return Some(attachment);
    }

    // line comments before jsx spread children should stay before the spread argument
    if comment_is_line
        && token_after_is_spread
        && let Some(target_node) = enclosing_owner
            .or(preceding_owner)
            .or(following_owner)
            .and_then(|owner| {
                promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Argument)
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // line comments after jsx spread operators should stay on the spread argument boundary
    if comment_is_line
        && token_before_is_spread
        && let Some(target_node) = enclosing_owner
            .or(preceding_owner)
            .or(following_owner)
            .and_then(|owner| {
                promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Argument)
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // line comments after label colons stay with the labelled statement owner
    if !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && token_before_is_colon
        && let Some(target_node) = preceding_owner
            .and_then(|owner| promote_owner_to_labelled_expression_ancestor(tree, parents, owner))
            .or_else(|| {
                following_owner.and_then(|owner| {
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
        && preceding_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Argument)
        && let Some(attachment) = attach_trailing_comma_close_brace_property_line_comment(
            tree,
            parents,
            token_before_span,
            preceding_owner,
        )
    {
        return Some(attachment);
    }

    // trailing line comments after JSX child expression containers should stay on the child
    if comment_is_line
        && !has_leading_newline
        && token_before_is_close_brace
        && token_after_is_less_than
        && let (Some(preceding_owner), Some(following_owner)) = (preceding_owner, following_owner)
        && tree.get_node_type(preceding_owner) == NodeType::Argument
        && tree.get_node_type(following_owner) == NodeType::Expression
        && matches!(
            tree.get(LocalNodeId::<Expression>::new(following_owner)),
            Expression::TreeExpression { .. }
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, preceding_owner);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // same-line comments after no-semi guards should stay on the guarded expression
    if token_before_is_semicolon
        && token_after_is_open_parenthesis
        && comment_is_star
        && let Some(mut target_node) = following_token_owner.or(following_owner)
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
        let target_node = following_token_owner.or(following_owner).map(|owner| {
            promote_owner_by_shared_start(tree, parents, owner, token_after_span.span.start)
        });
        if let Some(target_node) = target_node {
            // type-cast owners inside parenthesized wrappers should bind to the consuming group
            let mut target_node = target_node;
            if tree.get_node_type(target_node) == NodeType::Expression {
                let expression_id = LocalNodeId::<Expression>::new(target_node);
                let is_cast_like_type_binary = matches!(
                    tree.get(expression_id),
                    Expression::TypeBinary {
                        operator: ast::TypeBinaryOperator::Cast
                            | ast::TypeBinaryOperator::Satisfies,
                        ..
                    }
                );

                if is_cast_like_type_binary
                    && let Some(parent_id) = parents.get_by_id(target_node)
                    && tree.get_node_type(parent_id) == NodeType::Expression
                {
                    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                    let is_direct_parenthesized_wrapper = matches!(
                        tree.get(parent_expression_id),
                        Expression::Parenthesized { expression } if expression.id == target_node
                    );

                    if is_direct_parenthesized_wrapper {
                        if let Some(grandparent_id) = parents.get_by_id(parent_id)
                            && tree.get_node_type(grandparent_id) == NodeType::Expression
                        {
                            let grandparent_expression_id =
                                LocalNodeId::<Expression>::new(grandparent_id);
                            target_node = match tree.get(grandparent_expression_id) {
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
                                _ => target_node,
                            };
                        }
                    }
                }
            }

            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }
    }

    // inline comments after `{` before grouped keys and expressions stay on the rhs expression
    if is_inline_star_comment
        && token_before_is_open_brace
        && token_after_is_open_bracket_or_open_parenthesis
    {
        let target_node = following_token_owner.or(following_owner).map(|owner| {
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
    if is_inline_star_comment
        && token_before_is_spread
        && let Some(target_node) = enclosing_owner.or(preceding_owner).or(following_owner)
    {
        let target_node =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Parameter)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // declaration generic head seams should stay on declaration heads, not jsx trees
    let token_after_starts_tree_expression = [following_token_owner, following_owner]
        .into_iter()
        .flatten()
        .any(|owner| {
            let owner = promote_owner_to_tree_expression_parent(tree, parents, owner);
            tree.get_node_type(owner) == NodeType::Expression
                && matches!(
                    tree.get(LocalNodeId::<Expression>::new(owner)),
                    Expression::TreeExpression { .. }
                )
        });
    if !has_leading_newline
        && has_trailing_newline
        && token_after_is_less_than
        && !token_after_starts_tree_expression
    {
        let declaration_target = context
            .token_after
            .and_then(|token_after_index| {
                find_owner_at_or_after_token(tree, context.semantic_tokens, token_after_index)
            })
            .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            .or_else(|| {
                following_owner.and_then(|owner| {
                    promote_owner_to_declaration_ancestor(tree, parents, owner).or_else(|| {
                        (tree.get_node_type(owner) == NodeType::Declaration).then_some(owner)
                    })
                })
            })
            .or_else(|| {
                preceding_owner.and_then(|owner| {
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
        if comment_is_line
            && let Some(target_node) = token_before_span
                .and_then(|token| find_preferred_owner_starting_at(tree, token.span))
                .or(preceding_token_owner)
                .or(preceding_owner)
        {
            let target_node = normalize_owner_with_shared_end(
                tree,
                parents,
                target_node,
                token_before_source_span,
            );
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        if let Some(target_node) = enclosing_owner {
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
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_source_span);
        if has_trailing_newline {
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // inline block comments right after `(` in call and new expressions stay on the first argument
    if is_inline_star_comment
        && token_before_is_open_parenthesis
        && let Some(target_node) = enclosing_owner_first_dynamic_argument
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // inline block comments between call callees and `(` stay inside non-empty argument lists
    if is_inline_star_comment && token_after_is_open_parenthesis && enclosing_owner_is_call_or_new {
        if let Some(target_node) = enclosing_owner_first_dynamic_argument {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        if let Some(target_node) = preceding_token_owner.or(preceding_owner) {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfix));
        }
    }

    // comments before parenthesized call-style cast targets should bind to the call expression
    if is_inline_star_comment
        && token_after_is_open_parenthesis
        && preceding_owner.is_some_and(|owner| tree.get_node_type(owner) != NodeType::Expression)
    {
        if let Some(mut current_id) = following_owner {
            while let Some(parent_id) = parents.get_by_id(current_id) {
                if tree.get_node_type(parent_id) != NodeType::Expression {
                    current_id = parent_id;
                    continue;
                }

                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                let should_attach_to_call_like_parent = matches!(
                    tree.get(parent_expression_id),
                    Expression::Call { left, .. } | Expression::New { left, .. }
                        if left.id == current_id
                );
                if should_attach_to_call_like_parent {
                    return Some((Some(parent_id), AnnotationPosition::LinePrefix));
                }

                current_id = parent_id;
            }
        }
    }

    // comments between object open braces and computed keys stay inside the object
    if is_inline_star_comment && token_before_is_open_brace && token_after_is_open_bracket {
        if let Some(target_node) = preceding_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        if let Some(target_node) = enclosing_owner {
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
    }

    // line comments after logical operators belong to the right operand
    if is_trailing_line_comment
        && token_before_is_logical_operator
        && let Some(target_owner) = following_owner
    {
        let target_owner = token_after_source_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_owner, span.start))
            .unwrap_or(target_owner);
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    // trailing line comments after non-block if consequents stay on the consequent statement
    if is_trailing_line_comment
        && !token_before_is_close_parenthesis
        && !token_after_is_close_parenthesis
        && let Some(enclosing_owner) = enclosing_owner
        && let Some(then_owner) = if_expression_then_owner_without_else(tree, enclosing_owner)
    {
        let attachment = (Some(then_owner), AnnotationPosition::LinePostfixBoundary);
        return Some(attachment);
    }

    // line comments after ternary `:` stay with the consequent branch boundary
    if !has_leading_newline
        && comment_is_line
        && token_before_is_colon
        && !seam.token_before_is_return_type_colon
        && is_ternary_seam
        && let Some(attachment) = attach_line_comment_after_ternary_colon(
            tree,
            parents,
            preceding_owner,
            token_before_source_span,
        )
    {
        return Some(attachment);
    }

    // line comments before tree-expression container `}` stay on the container expression
    if !has_leading_newline
        && comment_is_line
        && token_after_is_close_brace
        && let Some(preceding_expression_owner) = preceding_owner
            .filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)
            .filter(|owner| {
                parents
                    .get_by_id(*owner)
                    .is_some_and(|parent_id| tree.get_node_type(parent_id) == NodeType::Argument)
            })
    {
        return Some((
            Some(preceding_expression_owner),
            AnnotationPosition::LinePostfixBoundary,
        ));
    }

    // comments immediately before `:` should stay with the branch expression shape:
    // tree expressions continue on the alternate side, scalar expressions stay on the consequent
    if comment_is_star
        && token_after_is_colon
        && is_ternary_seam
        && let Some(left_target_node) = preceding_token_owner.or(preceding_owner)
    {
        if has_leading_newline && let Some(target_owner) = ternary_following_owner {
            return Some((Some(target_owner), AnnotationPosition::LinePrefix));
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

        if left_is_tree_expression && let Some(target_owner) = ternary_following_owner {
            return Some((Some(target_owner), AnnotationPosition::LinePrefix));
        }

        if let Some(attachment) =
            attach_star_comment_before_ternary_colon(tree, Some(left_target_node), following_owner)
        {
            return Some(attachment);
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, left_target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // comments immediately after `?` should stay inside the consequent branch
    if comment_is_star
        && (token_before_is_maybe || token_before_is_colon)
        && is_ternary_seam
        && let Some(target_owner) = ternary_following_owner
    {
        let position = if has_leading_newline || has_trailing_newline {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_owner), position));
    }

    // inline comments before `)` in do-while conditions should stay on the condition expression
    if is_inline_star_comment
        && token_after_is_close_parenthesis
        && [
            preceding_token_owner,
            preceding_owner,
            following_owner,
            enclosing_owner,
            following_token_owner,
        ]
        .into_iter()
        .flatten()
        .any(|owner| owner_has_do_while_expression_ancestor(tree, parents, owner))
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // inline comments before `)` in ternary branches stay on the inner branch expression
    if is_inline_star_comment
        && token_after_is_close_parenthesis
        && is_ternary_seam
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // inline comments before `)` on grouped expressions should stay on the group
    if is_inline_star_comment
        && token_after_is_close_parenthesis
        && let Some(target_node) = preceding_token_owner
            .or(preceding_owner)
            .and_then(|owner| {
                promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
            })
            .or_else(|| {
                enclosing_owner.and_then(|owner| {
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

    // comments before closure-cast object literals stay with the rhs cast target
    if is_inline_star_comment
        && token_after_is_open_brace
        && token_before_is_open_parenthesis
        && let Some(mut target_node) = following_token_owner
            .or(following_owner)
            .or(enclosing_owner)
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
    if is_inline_star_comment
        && token_after_is_open_parenthesis
        && !(token_before_is_maybe && ternary_enclosing_owner.is_none())
        && !seam_is_new_signature_declaration
        && let Some(target_node) = following_token_owner
            .or(following_owner)
            .or(enclosing_owner)
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
        let target_node = if preceding_owner
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

    if let Some(attachment) = try_attach_comment_expression_operator(
        tree,
        parents,
        context,
        seam,
        enclosing_owner_cache,
        owners,
    ) {
        return Some(attachment);
    }

    None
}
