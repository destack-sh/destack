use ast::{
    AnnotationPosition, Expression, Keyword, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenSpan, TokenType,
};
use destack_ast as ast;
use destack_source::{File, Span};
use rustc_hash::FxHashMap;

use super::assignment::try_attach_comment_assignment;
use super::context::build_comment_attachment_setup;
use super::declaration::try_attach_comment_declaration;
use super::default::attach_comment_default;
use super::expression::try_attach_comment_expression;
use super::index::FormatterTriviaOwnerIndex;
use super::owner::{
    find_smallest_owner_enclosing_range, lowest_common_owner_ancestor,
    normalize_formatter_trivia_target_owner, promote_owner_to_node_type_ancestor,
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

    let owner_from_seam_tokens = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        });
    let owner_from_owner_pair =
        owners
            .left
            .zip(owners.right)
            .and_then(|(left_owner, right_owner)| {
                lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
            });
    let owner = owner_from_seam_tokens
        .into_iter()
        .chain(owner_from_owner_pair)
        .find(|owner_id| tree.get_node_type(*owner_id) == NodeType::Parameter)?;

    let target_node = normalize_formatter_trivia_target_owner(tree, owner);
    Some((Some(target_node), AnnotationPosition::BlockInfix))
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

/// Normalize line comments after object member trailing commas inside call arguments.
fn normalize_trailing_object_member_comment_attachment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &super::seam::CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    decision: CommentAttachmentDecision,
) -> CommentAttachmentDecision {
    let (owner, position) = decision;
    let Some(owner_id) = owner else {
        return (owner, position);
    };

    if !facts.comment_is_line
        || facts.has_leading_newline
        || !facts.token_before_is(TokenType::Comma)
        || !facts.token_after_is(TokenType::CloseBrace)
        || tree.get_node_type(owner_id) != NodeType::Argument
    {
        return (owner, position);
    }

    let member_owner = context.token_before_span.and_then(|comma_token| {
        let search_start = comma_token.span.start.saturating_sub(1);
        (search_start < comma_token.span.start).then(|| {
            find_smallest_owner_enclosing_range(tree, search_start, comma_token.span.start)
        })?
    });
    let Some(member_owner) = member_owner.and_then(|candidate| {
        promote_owner_to_node_type_ancestor(tree, parents, candidate, NodeType::Property)
    }) else {
        return (owner, position);
    };

    let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
    (Some(member_owner), AnnotationPosition::LinePostfixBoundary)
}

/// Resolve one comment trivia target owner and position from one token seam.
pub(in super::super) fn resolve_comment_trivia_attachment(
    file: &File,
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    trivia: destack_ast::CommentTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
) -> CommentAttachmentDecision {
    let setup = build_comment_attachment_setup(
        file,
        tree,
        semantic_tokens,
        token_keyword_by_span,
        trivia,
        owner_index,
        parents,
    );
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
        let decision = normalize_trailing_object_member_comment_attachment(
            tree, parents, &context, &facts, decision,
        );
        return decision;
    }

    let decision = attach_comment_default(&context, &facts, &mut seam_owner_cache, owners);
    let decision = normalize_trailing_object_member_comment_attachment(
        tree, parents, &context, &facts, decision,
    );
    decision
}
