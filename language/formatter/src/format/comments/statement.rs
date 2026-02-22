use ast::{
    AnnotationPosition, Block, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenType,
};
use destack_ast as ast;

use crate::format::comments::declaration::try_attach_comment_declaration_return_type_seam;
use crate::format::comments::owner::{
    find_smallest_owner_enclosing_token, lowest_common_owner_ancestor,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_to_node_type_ancestor, resolve_block_leading_comment_target,
};
use crate::format::comments::seam::{
    CommentAttachmentDecision, CommentAttachmentOwners, CommentSeamContext, CommentSeamFacts,
    CommentSeamKeyword, CommentSeamOwnerCache, resolve_comment_seam_owner,
};

/// Store statement routing flags shared by statement seam rules.
#[derive(Clone, Copy)]
struct StatementCommentRoutingFacts {
    /// Whether there is a newline before the comment.
    has_leading_newline: bool,
    /// Whether there is a newline after the comment.
    has_trailing_newline: bool,
    /// Whether the comment is one line.
    comment_is_line: bool,
    /// Whether the comment is a block comment.
    comment_is_star: bool,
    /// Whether the token before the comment is a comma.
    token_before_is_comma: bool,
    /// Whether the token before the comment is `)`.
    token_before_is_close_parenthesis: bool,
    /// Whether the token before the comment is `else`.
    token_before_is_else: bool,
    /// Whether the token after the comment is `{`.
    token_after_is_open_brace: bool,
    /// Whether the token after the comment is `)`.
    token_after_is_close_parenthesis: bool,
    /// Whether the token after the comment is `;`.
    token_after_is_semicolon: bool,
    /// Whether the token after the comment is `case` or `default`.
    token_after_is_case_or_default: bool,
}

/// Build statement routing facts from seam facts.
fn collect_statement_comment_routing_facts(
    facts: &CommentSeamFacts,
) -> StatementCommentRoutingFacts {
    StatementCommentRoutingFacts {
        has_leading_newline: facts.has_leading_newline,
        has_trailing_newline: facts.has_trailing_newline,
        comment_is_line: facts.comment_is_line,
        comment_is_star: facts.comment_is_star,
        token_before_is_comma: facts.token_before_is(TokenType::Comma),
        token_before_is_close_parenthesis: facts.token_before_is(TokenType::CloseParenthesis),
        token_before_is_else: facts.token_before_is_keyword(CommentSeamKeyword::Else),
        token_after_is_open_brace: facts.token_after_is(TokenType::OpenBrace),
        token_after_is_close_parenthesis: facts.token_after_is(TokenType::CloseParenthesis),
        token_after_is_semicolon: facts.token_after_is(TokenType::Semicolon),
        token_after_is_case_or_default: facts.token_after_is_case_or_default(),
    }
}

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
    routing_facts: StatementCommentRoutingFacts,
    target_owner: Option<u32>,
) -> Option<CommentAttachmentDecision> {
    if !routing_facts.comment_is_line
        || !routing_facts.token_before_is_comma
        || !routing_facts.token_after_is_close_parenthesis
    {
        return None;
    }

    let target_owner = target_owner?;
    let target_owner = normalize_parameter_or_argument_owner(tree, parents, target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve one own-line case/default prefix comment target.
fn resolve_case_or_default_prefix_target(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    right_owner: Option<u32>,
) -> Option<CommentAttachmentDecision> {
    let target_owner = resolve_comment_seam_owner(context, seam_owner_cache).or(right_owner)?;

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
        let (target_owner, position) = resolve_block_leading_comment_target(tree, block_id);
        return Some((Some(target_owner), position));
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Resolve one trailing line comment after a control-head `)` target.
fn resolve_control_head_line_comment_target(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    left_owner: Option<u32>,
    right_owner: Option<u32>,
) -> Option<CommentAttachmentDecision> {
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
                let (target_owner, position) =
                    resolve_block_leading_comment_target(tree, *block_id);
                return Some((Some(target_owner), position));
            }

            Some((Some(then_expression.id), AnnotationPosition::BlockPrefix))
        }
        Expression::While { body, .. }
        | Expression::ForEach { body, .. }
        | Expression::For { body, .. }
        | Expression::Loop { body } => {
            let (target_owner, position) = resolve_block_leading_comment_target(tree, *body);
            Some((Some(target_owner), position))
        }
        _ => None,
    }
}

/// Resolve statement-prefix seam comment rules.
pub(crate) fn try_attach_comment_statement_prefix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let routing_facts = collect_statement_comment_routing_facts(facts);
    let left_owner = owners.left;
    let right_owner = owners.right;
    let token_after_span = context.token_after_span;
    let token_before_span = context.token_before_span.map(|token| token.span);

    // own-line trailing separator comments before `)` should stay on the container item
    if routing_facts.has_leading_newline
        && let Some(decision) = try_attach_parameter_or_argument_separator_comment(
            tree,
            parents,
            routing_facts,
            left_owner,
        )
    {
        return Some(decision);
    }

    // own-line comments before semicolon guards stay with the guarded rhs expression
    if routing_facts.has_leading_newline && routing_facts.token_after_is_semicolon {
        let left_owner_is_statement_expression = left_owner.is_some_and(|owner| {
            tree.get_node_type(owner) == NodeType::Expression
                && matches!(
                    tree.get(LocalNodeId::<Expression>::new(owner)),
                    Expression::Statement(_)
                )
        });

        if (left_owner_is_statement_expression || left_owner.is_none())
            && let Some(mut target_owner) = token_after_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
                .or(right_owner)
        {
            if tree.get_node_type(target_owner) != NodeType::Expression
                && let Some(expression_target) = promote_owner_to_node_type_ancestor(
                    tree,
                    parents,
                    target_owner,
                    NodeType::Expression,
                )
            {
                target_owner = expression_target;
            }

            return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
        }

        if let Some(target_owner) = left_owner {
            let target_owner =
                normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
            return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // own-line comments before switch case labels should attach to the first case expression
    if routing_facts.has_leading_newline
        && routing_facts.token_after_is_case_or_default
        && let Some(decision) =
            resolve_case_or_default_prefix_target(tree, context, seam_owner_cache, right_owner)
    {
        return Some(decision);
    }

    None
}

/// Resolve statement-suffix seam comment rules.
pub(crate) fn try_attach_comment_statement_suffix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let routing_facts = collect_statement_comment_routing_facts(facts);
    let left_owner = owners.left;
    let right_owner = owners.right;

    // inline block comments between `else` and `{` stay with the else body block
    if !routing_facts.has_leading_newline
        && !routing_facts.has_trailing_newline
        && routing_facts.comment_is_star
        && routing_facts.token_before_is_else
        && routing_facts.token_after_is_open_brace
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
    if !routing_facts.has_leading_newline
        && routing_facts.has_trailing_newline
        && routing_facts.comment_is_line
        && routing_facts.token_before_is_close_parenthesis
        && !routing_facts.token_after_is_case_or_default
        && let Some(decision) =
            resolve_control_head_line_comment_target(tree, parents, left_owner, right_owner)
    {
        return Some(decision);
    }

    // return type seam comments should stay between `:` and the return type
    if let Some(decision) = try_attach_comment_declaration_return_type_seam(tree, facts, owners) {
        return Some(decision);
    }

    // parameter and argument trailing comments before `)` should stay on the container item
    if !routing_facts.has_leading_newline
        && routing_facts.has_trailing_newline
        && let Some(decision) = try_attach_parameter_or_argument_separator_comment(
            tree,
            parents,
            routing_facts,
            left_owner,
        )
    {
        return Some(decision);
    }

    // trailing comments after callback arguments should stay with the callback argument
    if !routing_facts.has_leading_newline
        && routing_facts.has_trailing_newline
        && routing_facts.comment_is_line
        && routing_facts.token_before_is_comma
        && !routing_facts.token_after_is_close_parenthesis
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
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let routing_facts = collect_statement_comment_routing_facts(facts);
    let right_owner = owners.right;

    // comments between method signatures and opening braces should stay inside the body
    if !routing_facts.has_leading_newline
        && routing_facts.has_trailing_newline
        && routing_facts.token_after_is_open_brace
        && let Some(target_owner) = right_owner
    {
        if tree.get_node_type(target_owner) == NodeType::Block {
            let block_id = LocalNodeId::<Block>::new(target_owner);
            let (target_owner, position) = resolve_block_leading_comment_target(tree, block_id);
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
