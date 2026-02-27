use destack_ast::{
    AnnotationPosition, Block, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenSpan, TokenType, WhileKind,
};

use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner,
};
use super::declaration::try_attach_comment_declaration_return_type_seam;
use super::ownership::{
    block_leading_comment_target, find_smallest_owner_enclosing_token,
    lowest_common_owner_ancestor, normalize_formatter_trivia_target_owner,
    promote_owner_to_node_type_ancestor,
};
use super::semicolon::attach_semicolon_guard_own_line_comment;

/// Normalize one owner to its parameter or argument container owner.
fn normalize_parameter_or_argument_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> u32 {
    promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Parameter)
        .or_else(|| {
            promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Argument)
        })
        .unwrap_or(owner_id)
}

/// Normalize one owner to its argument container owner.
fn normalize_argument_owner(tree: &NodeTree, parents: &NodeParentIndex, owner_id: u32) -> u32 {
    promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Argument)
        .unwrap_or(owner_id)
}

/// Return whether one owner token sits inside one decorator expression ancestry.
fn seam_follows_decorator_head(context: &CommentSeamContext<'_>) -> bool {
    let Some(mut token_index) = context.token_before else {
        return false;
    };

    loop {
        let token_type = context.semantic_tokens[token_index].token.ty;
        if token_type == TokenType::At {
            return true;
        }

        if token_type == TokenType::Newline || token_index == 0 {
            return false;
        }

        token_index -= 1;
    }
}

/// Resolve one else-body owner for comments between `else` and its body.
fn else_body_comment_target_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    following_owner: Option<u32>,
) -> Option<u32> {
    let target_owner = following_owner?;
    let target_owner = if tree.get_node_type(target_owner) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(target_owner);
        if let Expression::Block(block_id) = tree.get(expression_id) {
            block_id.id
        } else {
            target_owner
        }
    } else {
        target_owner
    };
    let target_owner =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Block)
            .unwrap_or(target_owner);

    Some(target_owner)
}

/// Attach one separator line comment before `)` to its parameter or argument owner.
fn try_attach_parameter_or_argument_separator_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    target_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.comment_is_line
        || !seam.token_before_is(TokenType::Comma)
        || !seam.token_after_is(TokenType::CloseParenthesis)
    {
        return None;
    }

    let target_owner = target_owner?;
    let target_owner = normalize_parameter_or_argument_owner(tree, parents, target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve one own line case/default prefix comment target.
fn case_or_default_prefix_target(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if let Some(following_owner) = following_owner
        && tree.get_node_type(following_owner) == NodeType::MatchCase
    {
        return Some((Some(following_owner), AnnotationPosition::BlockPrefix));
    }

    let target_owner =
        comment_enclosing_owner(context, enclosing_owner_cache).or(following_owner)?;

    if tree.get_node_type(target_owner) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(target_owner);
        if let Expression::Match { cases, .. } = tree.get(expression_id)
            && let Some(first_case) = cases.first().copied()
        {
            return Some((Some(first_case.id), AnnotationPosition::BlockPrefix));
        }
    }

    if tree.get_node_type(target_owner) == NodeType::Block {
        let block_id = LocalNodeId::<Block>::new(target_owner);
        let (target_owner, position) = block_leading_comment_target(tree, block_id);
        return Some((Some(target_owner), position));
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Promote one owner to the nearest control-head expression ancestor.
fn promote_owner_to_control_head_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<LocalNodeId<Expression>> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            match tree.get(expression_id) {
                Expression::If { .. }
                | Expression::ForEach { .. }
                | Expression::For { .. }
                | Expression::Loop { .. } => return Some(expression_id),
                Expression::While { kind, .. } if *kind != WhileKind::DoWhile => {
                    return Some(expression_id);
                }
                _ => {}
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Resolve one comment after a control-head `)` target.
fn control_head_comment_attachment_for_expression(
    tree: &NodeTree,
    control_expression: LocalNodeId<Expression>,
) -> Option<CommentAttachment> {
    match tree.get(control_expression) {
        Expression::If {
            then_expression, ..
        } => {
            if matches!(tree.get(*then_expression), Expression::Block(_)) {
                let Expression::Block(block_id) = tree.get(*then_expression) else {
                    unreachable!();
                };
                let (target_owner, position) = block_leading_comment_target(tree, *block_id);
                return Some((Some(target_owner), position));
            }

            Some((Some(then_expression.id), AnnotationPosition::BlockPrefix))
        }
        Expression::While { body, .. }
        | Expression::ForEach { body, .. }
        | Expression::For { body, .. }
        | Expression::Loop { body } => {
            let (target_owner, position) = block_leading_comment_target(tree, *body);
            Some((Some(target_owner), position))
        }
        _ => None,
    }
}

/// Resolve one comment after a control-head token span.
fn control_head_comment_target_from_token(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    token_before_span: Option<TokenSpan>,
) -> Option<CommentAttachment> {
    let token_before_span = token_before_span?;
    let owner_id = find_smallest_owner_enclosing_token(tree, token_before_span.span)?;
    let control_expression =
        promote_owner_to_control_head_expression_ancestor(tree, parents, owner_id)?;

    control_head_comment_attachment_for_expression(tree, control_expression)
}

/// Resolve one comment after a control-head `)` target.
fn control_head_comment_target(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let shared_owner =
        preceding_owner
            .zip(following_owner)
            .and_then(|(preceding_owner, following_owner)| {
                lowest_common_owner_ancestor(tree, parents, preceding_owner, following_owner)
            });

    let control_expression = [shared_owner, preceding_owner]
        .into_iter()
        .flatten()
        .find_map(|owner_id| {
            promote_owner_to_control_head_expression_ancestor(tree, parents, owner_id)
        })?;

    control_head_comment_attachment_for_expression(tree, control_expression)
}

/// Resolve statement-prefix seam comment rules.
pub(crate) fn try_attach_comment_statement_prefix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;

    // own-line comments between decorators and decorated items stay with the decorated owner
    let comment_starts_after_token_before_newline =
        context.token_before_span.is_some_and(|token| {
            context
                .file
                .is_same_line(token.span.end.saturating_sub(1), context.trivia.span.start)
        });
    if seam.comment_is_line
        && !comment_starts_after_token_before_newline
        && seam_follows_decorator_head(context)
    {
        let target_owner = following_owner
            .or_else(|| {
                context
                    .token_after_span
                    .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            })
            .and_then(|owner| {
                promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Parameter)
                    .or_else(|| {
                        promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
                    })
                    .or_else(|| {
                        promote_owner_to_node_type_ancestor(
                            tree,
                            parents,
                            owner,
                            NodeType::Property,
                        )
                    })
                    .or_else(|| {
                        promote_owner_to_node_type_ancestor(
                            tree,
                            parents,
                            owner,
                            NodeType::Declaration,
                        )
                    })
            });
        if let Some(target_owner) = target_owner {
            let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
            return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
        }
    }

    // own line trailing separator comments before `)` should stay on the container item
    if seam.has_leading_newline
        && let Some(attachment) =
            try_attach_parameter_or_argument_separator_comment(tree, parents, seam, preceding_owner)
    {
        return Some(attachment);
    }

    // own line comments between `else` and its body should stay with the else body
    if seam.has_leading_newline
        && seam.token_before_is_keyword(CommentSeamKeyword::Else)
        && let Some(target_owner) = else_body_comment_target_owner(tree, parents, following_owner)
    {
        let position = if tree.get_node_type(target_owner) == NodeType::Block {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_owner), position));
    }

    // own line comments after control heads should stay before the body statement
    if seam.has_leading_newline
        && seam.token_before_is(TokenType::CloseParenthesis)
        && !seam.token_after_is_case_or_default()
        && (seam.comment_is_line
            || seam.token_after_is(TokenType::Semicolon)
            || !seam.token_after_is(TokenType::OpenBrace))
        && let Some(attachment) =
            control_head_comment_target_from_token(tree, parents, context.token_before_span)
                .or_else(|| {
                    control_head_comment_target(tree, parents, preceding_owner, following_owner)
                })
    {
        return Some(attachment);
    }

    // own line comments before semicolon guards stay with the guarded following expression
    if seam.token_after_is(TokenType::Semicolon)
        && let Some(attachment) =
            attach_semicolon_guard_own_line_comment(tree, parents, context, seam, owners)
    {
        return Some(attachment);
    }

    // own line comments between switch labels should stay with the following label
    if seam.has_leading_newline
        && seam.token_before_is(TokenType::Colon)
        && seam.token_after_is_case_or_default()
    {
        let target_owner = context
            .token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .and_then(|owner_id| {
                promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::MatchCase)
                    .or(Some(owner_id))
            })
            .map(|owner_id| normalize_formatter_trivia_target_owner(tree, owner_id));

        if let Some(target_owner) = target_owner {
            return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
        }
    }

    // own line comments before switch case labels should attach to the first case expression
    if seam.has_leading_newline
        && seam.token_after_is_case_or_default()
        && let Some(attachment) =
            case_or_default_prefix_target(tree, context, enclosing_owner_cache, following_owner)
    {
        return Some(attachment);
    }

    None
}

/// Resolve statement-suffix seam comment rules.
pub(crate) fn try_attach_comment_statement_suffix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;

    // inline line comments between `else` and its body stay with the else body
    if !seam.has_leading_newline
        && seam.comment_is_line
        && seam.token_before_is_keyword(CommentSeamKeyword::Else)
        && let Some(target_owner) = else_body_comment_target_owner(tree, parents, following_owner)
    {
        let position = if tree.get_node_type(target_owner) == NodeType::Block {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_owner), position));
    }

    // inline block comments between `else` and its body stay with the else body
    if !seam.has_leading_newline
        && seam.comment_is_star
        && seam.token_before_is_keyword(CommentSeamKeyword::Else)
        && !matches!(
            seam.token_after_type,
            Some(
                TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            )
        )
        && let Some(target_owner) = else_body_comment_target_owner(tree, parents, following_owner)
    {
        if tree.get_node_type(target_owner) == NodeType::Block {
            return Some((Some(target_owner), AnnotationPosition::LinePrefix));
        }

        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    // same-line comments after control heads should stay before the body statement
    if !seam.has_leading_newline
        && seam.token_before_is(TokenType::CloseParenthesis)
        && !seam.token_after_is_case_or_default()
        && (seam.comment_is_line
            || seam.token_after_is(TokenType::Semicolon)
            || !seam.token_after_is(TokenType::OpenBrace))
        && let Some(attachment) =
            control_head_comment_target_from_token(tree, parents, context.token_before_span)
                .or_else(|| {
                    control_head_comment_target(tree, parents, preceding_owner, following_owner)
                })
    {
        if seam.comment_is_star
            && !seam.has_trailing_newline
            && !seam.token_after_is(TokenType::Semicolon)
            && let Some(target_owner) = attachment.0
            && tree.get_node_type(target_owner) != NodeType::Block
        {
            return Some((Some(target_owner), AnnotationPosition::LinePrefix));
        }

        return Some(attachment);
    }

    // trailing line comments after `switch (...) {` should stay on the following case label seam
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && seam.comment_is_line
        && (seam.token_before_is(TokenType::OpenBrace)
            || seam.token_before_is(TokenType::CloseParenthesis))
        && seam.token_after_is_case_or_default()
    {
        let target_owner = following_owner.or_else(|| {
            let preceding_owner = preceding_owner?;
            if tree.get_node_type(preceding_owner) != NodeType::Expression {
                return None;
            }

            let expression_id = LocalNodeId::<Expression>::new(preceding_owner);
            let Expression::Match { cases, .. } = tree.get(expression_id) else {
                return None;
            };

            cases.first().map(|case_id| case_id.id)
        });

        if let Some(target_owner) = target_owner {
            let target_owner = promote_owner_to_node_type_ancestor(
                tree,
                parents,
                target_owner,
                NodeType::MatchCase,
            )
            .unwrap_or(target_owner);
            let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
            return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
        }
    }

    // trailing line comments after switch label `:` should stay before the case body
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && seam.comment_is_line
        && seam.token_before_is(TokenType::Colon)
        && !seam.token_after_is_case_or_default()
    {
        let in_match_case = preceding_owner
            .and_then(|owner| {
                promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::MatchCase)
            })
            .is_some()
            || following_owner
                .and_then(|owner| {
                    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::MatchCase)
                })
                .is_some();
        if in_match_case {
            let target_owner = following_owner
                .or(preceding_owner)
                .map(|owner| normalize_formatter_trivia_target_owner(tree, owner));
            if let Some(target_owner) = target_owner {
                return Some((Some(target_owner), AnnotationPosition::LinePrefix));
            }
        }
    }

    // return type seam comments should stay between `:` and the return type
    if let Some(attachment) = try_attach_comment_declaration_return_type_seam(tree, seam, owners) {
        return Some(attachment);
    }

    // parameter and argument trailing comments before `)` should stay on the container item
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && let Some(attachment) =
            try_attach_parameter_or_argument_separator_comment(tree, parents, seam, preceding_owner)
    {
        return Some(attachment);
    }

    // trailing comments after callback arguments should stay with the callback argument
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && seam.comment_is_line
        && seam.token_before_is(TokenType::Comma)
        && !seam.token_after_is(TokenType::CloseParenthesis)
        && let Some(target_owner) = preceding_owner
        && promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Argument)
            .is_some()
    {
        let target_owner = normalize_argument_owner(tree, parents, target_owner);
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Resolve block body seam comment rules.
pub(crate) fn try_attach_comment_block_body(
    tree: &NodeTree,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let following_owner = owners.following;

    // comments between method signatures and opening braces should stay inside the body
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && seam.token_after_is(TokenType::OpenBrace)
        && let Some(target_owner) = following_owner
    {
        if tree.get_node_type(target_owner) == NodeType::Block {
            let block_id = LocalNodeId::<Block>::new(target_owner);
            let (target_owner, position) = block_leading_comment_target(tree, block_id);
            return Some((Some(target_owner), position));
        }

        if tree.get_node_type(target_owner) == NodeType::Declaration {
            return Some((Some(target_owner), AnnotationPosition::BlockInfix));
        }

        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    None
}
