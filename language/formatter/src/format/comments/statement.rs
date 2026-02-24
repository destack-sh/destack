use destack_ast::{
    AnnotationPosition, Block, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenType,
};

use crate::format::comments::boundary::{
    CommentAttachment, CommentAttachmentOwners, CommentSeamContext, CommentSeamData,
    CommentSeamKeyword, CommentSeamOwnerCache, comment_seam_owner,
    previous_non_newline_token_index,
};
use crate::format::comments::declaration::try_attach_comment_declaration_return_type_seam;
use crate::format::comments::ownership::{
    block_leading_comment_target, find_smallest_owner_enclosing_token,
    lowest_common_owner_ancestor, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_to_node_type_ancestor,
};

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
    seam_owner_cache: &mut CommentSeamOwnerCache,
    right_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let target_owner = comment_seam_owner(context, seam_owner_cache).or(right_owner)?;

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

/// Resolve one trailing line comment after a control-head `)` target.
fn control_head_line_comment_target(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    left_owner: Option<u32>,
    right_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner) else {
        return None;
    };

    let shared_owner = lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)?;
    if tree.get_node_type(shared_owner) != NodeType::Expression {
        return None;
    }

    let shared_expression = LocalNodeId::<Expression>::new(shared_owner);
    match tree.get(shared_expression) {
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

/// Resolve one own-line semicolon-guard comment across both seam shapes.
pub(crate) fn try_attach_comment_semicolon_guard_own_line(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline || !seam.comment_is_line {
        return None;
    }
    if !seam.token_before_is(TokenType::Semicolon) {
        return None;
    }
    if !matches!(
        seam.token_after_type,
        Some(TokenType::OpenBracket | TokenType::OpenParenthesis)
    ) {
        return None;
    }

    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span;
    let left_owner_from_previous_non_newline_token = context
        .token_before
        .and_then(|token_index| {
            previous_non_newline_token_index(context.semantic_tokens, token_index)
        })
        .and_then(|token_index| context.semantic_tokens.get(token_index))
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let left_semicolon_owner = owners.left.or(left_owner_from_previous_non_newline_token);
    let right_expression_owner = token_after_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        .or(owners.right)
        .and_then(|target_owner| {
            if tree.get_node_type(target_owner) == NodeType::Expression {
                Some(target_owner)
            } else {
                promote_owner_to_node_type_ancestor(
                    tree,
                    parents,
                    target_owner,
                    NodeType::Expression,
                )
            }
        });
    let left_owner_is_statement_expression = left_semicolon_owner.is_some_and(|owner| {
        tree.get_node_type(owner) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(owner)),
                Expression::Statement(_)
            )
    });
    let left_owner_is_declaration_or_member = left_semicolon_owner.is_some_and(|owner| {
        matches!(
            tree.get_node_type(owner),
            NodeType::Declaration | NodeType::Member
        ) || promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Declaration)
            .is_some()
            || promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member).is_some()
    });
    let comment_column = context
        .file
        .get_position(context.trivia.span.start)
        .map_or(1, |(_, column)| column);
    let should_keep_left_owner = left_owner_is_statement_expression
        || left_owner_is_declaration_or_member
        || comment_column > 1;

    if let Some(target_owner) = left_semicolon_owner
        && should_keep_left_owner
    {
        let mut target_owner =
            normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
        if let Some(member_owner) =
            promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Member)
        {
            target_owner = member_owner;
        } else if let Some(declaration_owner) =
            promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Declaration)
        {
            target_owner = declaration_owner;
        }

        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    if let Some(target_owner) = right_expression_owner {
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    if let Some(target_owner) = left_semicolon_owner {
        let target_owner =
            normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Resolve statement-prefix seam comment rules.
pub(crate) fn try_attach_comment_statement_prefix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachment> {
    let left_owner = owners.left;
    let right_owner = owners.right;

    // own line trailing separator comments before `)` should stay on the container item
    if seam.has_leading_newline
        && let Some(attachment) =
            try_attach_parameter_or_argument_separator_comment(tree, parents, seam, left_owner)
    {
        return Some(attachment);
    }

    // own line line comments after control heads should stay before the body statement
    if seam.has_leading_newline
        && seam.comment_is_line
        && seam.token_before_is(TokenType::CloseParenthesis)
        && !seam.token_after_is(TokenType::Semicolon)
        && !seam.token_after_is_case_or_default()
        && let Some(attachment) =
            control_head_line_comment_target(tree, parents, left_owner, right_owner)
    {
        return Some(attachment);
    }

    // own line comments before semicolon guards stay with the guarded right expression
    if seam.token_after_is(TokenType::Semicolon)
        && let Some(attachment) =
            try_attach_comment_semicolon_guard_own_line(tree, parents, context, seam, owners)
    {
        return Some(attachment);
    }

    // own line comments before switch case labels should attach to the first case expression
    if seam.has_leading_newline
        && seam.token_after_is_case_or_default()
        && let Some(attachment) =
            case_or_default_prefix_target(tree, context, seam_owner_cache, right_owner)
    {
        return Some(attachment);
    }

    None
}

/// Resolve statement-suffix seam comment rules.
pub(crate) fn try_attach_comment_statement_suffix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachment> {
    let left_owner = owners.left;
    let right_owner = owners.right;

    // inline block comments between `else` and `{` stay with the else body block
    if !seam.has_leading_newline
        && !seam.has_trailing_newline
        && seam.comment_is_star
        && seam.token_before_is_keyword(CommentSeamKeyword::Else)
        && seam.token_after_is(TokenType::OpenBrace)
        && let Some(target_owner) = right_owner
    {
        let target_owner =
            promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Block)
                .unwrap_or(target_owner);
        if tree.get_node_type(target_owner) == NodeType::Block {
            return Some((Some(target_owner), AnnotationPosition::LinePrefix));
        }

        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    // trailing line comments after control heads should stay before the body statement
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && seam.comment_is_line
        && seam.token_before_is(TokenType::CloseParenthesis)
        && !seam.token_after_is(TokenType::Semicolon)
        && !seam.token_after_is_case_or_default()
        && let Some(attachment) =
            control_head_line_comment_target(tree, parents, left_owner, right_owner)
    {
        return Some(attachment);
    }

    // return type seam comments should stay between `:` and the return type
    if let Some(attachment) = try_attach_comment_declaration_return_type_seam(tree, seam, owners) {
        return Some(attachment);
    }

    // parameter and argument trailing comments before `)` should stay on the container item
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && let Some(attachment) =
            try_attach_parameter_or_argument_separator_comment(tree, parents, seam, left_owner)
    {
        return Some(attachment);
    }

    // trailing comments after callback arguments should stay with the callback argument
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && seam.comment_is_line
        && seam.token_before_is(TokenType::Comma)
        && !seam.token_after_is(TokenType::CloseParenthesis)
        && let Some(target_owner) = left_owner
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
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachment> {
    let right_owner = owners.right;

    // comments between method signatures and opening braces should stay inside the body
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && seam.token_after_is(TokenType::OpenBrace)
        && let Some(target_owner) = right_owner
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
