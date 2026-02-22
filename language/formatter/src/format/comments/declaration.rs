use ast::{AnnotationPosition, NodeParentIndex, NodeTree, NodeType, TokenType};
use destack_ast as ast;

use crate::format::comments::index::FormatterTriviaOwnerIndex;
use crate::format::comments::owner::{
    find_next_declaration_owner_from_token, find_next_member_owner_from_token,
    find_smallest_owner_enclosing_range, normalize_formatter_trivia_target_owner,
    promote_owner_to_declaration_ancestor, promote_owner_to_node_type_ancestor,
};
use crate::format::comments::seam::{
    CommentAttachmentDecision, CommentAttachmentOwners, CommentSeamContext, CommentSeamFacts,
    CommentSeamKeyword,
};

/// Return whether a declaration owner supports inline head comments before `{`.
fn declaration_owner_has_head_before_open_brace(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Declaration {
        return false;
    }

    let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(owner_id);
    matches!(
        tree.get(declaration_id),
        ast::Declaration::Class { .. }
            | ast::Declaration::Struct { .. }
            | ast::Declaration::Enum { .. }
            | ast::Declaration::Interface { .. }
            | ast::Declaration::Extension { .. }
            | ast::Declaration::Namespace { .. }
            | ast::Declaration::Global { .. }
    )
}

/// Return whether one declaration owner is a `new (...) => ...` function signature.
fn declaration_owner_is_new_signature(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Declaration {
        return false;
    }

    let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(owner_id);
    let ast::Declaration::Function { signature, .. } = tree.get(declaration_id) else {
        return false;
    };

    signature.mode == Some(ast::FunctionMode::New)
}

/// Promote one owner to the nearest member-like owner.
fn promote_owner_to_member_like_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Member).or_else(|| {
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Property)
    })
}

/// Resolve inline block comments around member optional `?` seams.
pub(crate) fn try_attach_comment_declaration_optional_member_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if facts.has_leading_newline || facts.has_trailing_newline || !facts.comment_is_star {
        return None;
    }

    if !facts.token_after_is(TokenType::Maybe) && !facts.token_before_is(TokenType::Maybe) {
        return None;
    }

    let seam_owner = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        });

    let target_node = seam_owner
        .and_then(|owner| promote_owner_to_member_like_ancestor(tree, parents, owner))
        .or_else(|| {
            owners
                .left
                .and_then(|owner| promote_owner_to_member_like_ancestor(tree, parents, owner))
        })
        .or_else(|| {
            owners
                .right
                .and_then(|owner| promote_owner_to_member_like_ancestor(tree, parents, owner))
        })?;

    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Resolve inline block comments between `new` and constructor type parameter lists.
pub(crate) fn try_attach_comment_declaration_new_signature_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if facts.has_leading_newline
        || facts.has_trailing_newline
        || !facts.comment_is_star
        || !facts.token_after_is(TokenType::OpenParenthesis)
    {
        return None;
    }

    let seam_owner = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        });

    let target_node = seam_owner
        .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        .or_else(|| {
            owners
                .left
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })
        .or_else(|| {
            owners
                .right
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })
        .filter(|owner| declaration_owner_is_new_signature(tree, *owner))?;

    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Resolve declaration `implements` own-line seam comments.
pub(crate) fn try_attach_comment_declaration_implements_seam(
    tree: &NodeTree,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if !facts.has_leading_newline {
        return None;
    }

    if !facts.token_before_is_keyword(CommentSeamKeyword::Implements) {
        return None;
    }

    let target_node = owners.left?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockPostfix))
}

/// Resolve declaration head comments directly before `{`.
pub(crate) fn try_attach_comment_declaration_head_open_brace_seam(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if facts.has_leading_newline || !facts.comment_is_star {
        return None;
    }

    if !facts.token_after_is(TokenType::OpenBrace) {
        return None;
    }

    let target_node = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        })
        .filter(|owner| declaration_owner_has_head_before_open_brace(tree, *owner))
        .or_else(|| {
            owners
                .left
                .filter(|owner| declaration_owner_has_head_before_open_brace(tree, *owner))
        })?;

    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Resolve declaration decorator-adjacent seam comments.
pub(crate) fn try_attach_comment_declaration_decorator_seam(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if !facts.token_after_is(TokenType::At) {
        return None;
    }

    let member_target_from_token = context.token_after.and_then(|token_after_index| {
        find_next_member_owner_from_token(tree, owner_index, token_after_index)
    });
    let member_target = owners.right.and_then(|owner| {
        promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
    });
    let declaration_target = context
        .token_after
        .and_then(|token_after_index| {
            find_next_declaration_owner_from_token(tree, owner_index, token_after_index)
        })
        .or_else(|| {
            owners
                .right
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })
        .or_else(|| {
            owners
                .left
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })
        .or_else(|| {
            owners
                .right
                .filter(|owner| tree.get_node_type(*owner) == NodeType::Declaration)
        })
        .or_else(|| {
            owners
                .left
                .filter(|owner| tree.get_node_type(*owner) == NodeType::Declaration)
        });

    let target_node = member_target_from_token
        .or(member_target)
        .or(declaration_target)?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    if facts.has_leading_newline {
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Resolve declaration comments between parameter lists and arrows.
pub(crate) fn try_attach_comment_declaration_arrow_seam(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
) -> Option<CommentAttachmentDecision> {
    if !facts.comment_is_star {
        return None;
    }

    if !matches!(
        facts.token_after_type,
        Some(TokenType::Arrow | TokenType::ArrowWide)
    ) {
        return None;
    }

    let (token_before_span, token_after_span) =
        (context.token_before_span?, context.token_after_span?);
    let target_node = find_smallest_owner_enclosing_range(
        tree,
        token_before_span.span.start,
        token_after_span.span.end,
    )?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Resolve declaration export-head seam comments.
pub(crate) fn try_attach_comment_declaration_export_seam(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
) -> Option<CommentAttachmentDecision> {
    if !facts.has_trailing_newline {
        return None;
    }

    if !facts.token_before_is_keyword(CommentSeamKeyword::Export) {
        return None;
    }

    let target_node = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        })?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Resolve declaration return-type seam comments between `:` and the return type.
pub(crate) fn try_attach_comment_declaration_return_type_seam(
    tree: &NodeTree,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if facts.has_leading_newline || !facts.has_trailing_newline || !facts.comment_is_line {
        return None;
    }

    if !facts.token_before_is_return_type_colon {
        return None;
    }

    let target_node = owners.right?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Resolve declaration seam comment rules.
pub(crate) fn try_attach_comment_declaration(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    if let Some(decision) =
        try_attach_comment_declaration_optional_member_seam(tree, parents, context, facts, owners)
    {
        return Some(decision);
    }

    if let Some(decision) =
        try_attach_comment_declaration_new_signature_seam(tree, parents, context, facts, owners)
    {
        return Some(decision);
    }

    if let Some(decision) = try_attach_comment_declaration_implements_seam(tree, facts, owners) {
        return Some(decision);
    }

    if let Some(decision) =
        try_attach_comment_declaration_head_open_brace_seam(tree, context, facts, owners)
    {
        return Some(decision);
    }

    if let Some(decision) = try_attach_comment_declaration_decorator_seam(
        tree,
        owner_index,
        parents,
        context,
        facts,
        owners,
    ) {
        return Some(decision);
    }

    if let Some(decision) = try_attach_comment_declaration_arrow_seam(tree, context, facts) {
        return Some(decision);
    }

    if let Some(decision) = try_attach_comment_declaration_export_seam(tree, context, facts) {
        return Some(decision);
    }

    None
}
