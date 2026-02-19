use ast::{
    AnnotationPosition, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenSpan,
    TokenType,
};
use destack_ast as ast;
use destack_source::File;

use super::assignment::try_attach_comment_assignment;
use super::context::build_comment_attachment_setup;
use super::declaration::try_attach_comment_declaration;
use super::default::attach_comment_default;
use super::expression::try_attach_comment_expression;
use super::index::FormatterTriviaOwnerIndex;
use super::owner::{
    find_smallest_owner_enclosing_range, lowest_common_owner_ancestor,
    normalize_formatter_trivia_target_owner,
};
use super::seam::{CommentAttachmentDecision, CommentSeamFacts, CommentSeamOwnerCache};
use super::statement::{
    try_attach_comment_block_body, try_attach_comment_statement_prefix,
    try_attach_comment_statement_suffix,
};
use super::token::{delimiters_match, is_close_delimiter_token, is_open_delimiter_token};

/// Try to attach one comment inside matching delimiters as container infix trivia.
fn try_attach_comment_delimiter_interior(
    tree: &NodeTree,
    context: &super::seam::CommentSeamContext<'_>,
) -> Option<CommentAttachmentDecision> {
    let token_before_span = context.token_before_span;
    let token_after_span = context.token_after_span;
    let (Some(token_before_span), Some(token_after_span)) = (token_before_span, token_after_span)
    else {
        return None;
    };

    if !is_open_delimiter_token(token_before_span.token.ty)
        || !is_close_delimiter_token(token_after_span.token.ty)
        || !delimiters_match(token_before_span.token.ty, token_after_span.token.ty)
    {
        return None;
    }

    let container_owner = find_smallest_owner_enclosing_range(
        tree,
        token_before_span.span.start,
        token_after_span.span.end,
    )?;

    let target_node = if tree.get_node_type(container_owner) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(container_owner);
        if let Expression::Block(block_id) = tree.get(expression_id) {
            block_id.id
        } else {
            normalize_formatter_trivia_target_owner(tree, container_owner)
        }
    } else {
        normalize_formatter_trivia_target_owner(tree, container_owner)
    };

    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Try to attach one seam comment between parameter name and type to the parameter owner.
fn try_attach_comment_parameter_type_boundary(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &super::seam::CommentSeamContext<'_>,
    owners: super::seam::CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if !context
        .token_after_span
        .is_some_and(|token| token.token.ty == TokenType::Colon)
    {
        return None;
    }

    let (Some(left_owner), Some(right_owner)) = (owners.left, owners.right) else {
        return None;
    };
    let owner = lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)?;
    if tree.get_node_type(owner) != NodeType::Parameter {
        return None;
    }

    let target_node = normalize_formatter_trivia_target_owner(tree, owner);
    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Try specialized seam attachment handlers in priority order.
fn try_attach_comment_with_specialized_handlers(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &super::seam::CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: super::seam::CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if let Some(decision) = try_attach_comment_expression(
        tree,
        owner_index,
        parents,
        context,
        facts,
        seam_owner_cache,
        owners,
    ) {
        return Some(decision);
    }

    if let Some(decision) =
        try_attach_comment_statement_prefix(tree, parents, context, facts, seam_owner_cache, owners)
    {
        return Some(decision);
    }

    if let Some(decision) =
        try_attach_comment_declaration(tree, owner_index, parents, context, facts, owners)
    {
        return Some(decision);
    }

    if let Some(decision) = try_attach_comment_statement_suffix(tree, parents, facts, owners) {
        return Some(decision);
    }

    if let Some(decision) =
        try_attach_comment_assignment(tree, parents, context, facts, seam_owner_cache, owners)
    {
        return Some(decision);
    }

    if let Some(decision) = try_attach_comment_block_body(tree, facts, owners) {
        return Some(decision);
    }

    None
}

/// Resolve one comment trivia target owner and position from one token seam.
pub(in super::super) fn resolve_comment_trivia_attachment(
    file: &File,
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    trivia: destack_ast::CommentTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
) -> CommentAttachmentDecision {
    let setup =
        build_comment_attachment_setup(file, tree, semantic_tokens, trivia, owner_index, parents);
    let context = setup.context;
    let owners = setup.owners;

    if let Some(decision) = try_attach_comment_delimiter_interior(tree, &context) {
        return decision;
    }

    if let Some(decision) =
        try_attach_comment_parameter_type_boundary(tree, parents, &context, owners)
    {
        return decision;
    }

    let facts = CommentSeamFacts::build(&context);
    let mut seam_owner_cache = CommentSeamOwnerCache::default();

    if let Some(decision) = try_attach_comment_with_specialized_handlers(
        tree,
        owner_index,
        parents,
        &context,
        &facts,
        &mut seam_owner_cache,
        owners,
    ) {
        return decision;
    }

    attach_comment_default(&context, &facts, &mut seam_owner_cache, owners)
}
