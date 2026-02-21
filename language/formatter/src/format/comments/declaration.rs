use ast::{AnnotationPosition, NodeParentIndex, NodeTree, NodeType, TokenType};
use destack_ast as ast;

use super::index::FormatterTriviaOwnerIndex;
use super::owner::{
    find_next_declaration_owner_from_token, find_next_member_owner_from_token,
    find_smallest_owner_enclosing_range, normalize_formatter_trivia_target_owner,
    promote_owner_to_declaration_ancestor, promote_owner_to_node_type_ancestor,
};
use super::seam::{
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

/// Resolve declaration seam comment rules.
pub(super) fn try_attach_comment_declaration(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    facts: &CommentSeamFacts,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachmentDecision> {
    let left_owner = owners.left;
    let right_owner = owners.right;
    let token_after = context.token_after;
    let token_before_span = context.token_before_span;
    let token_after_span = context.token_after_span;
    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_star = facts.comment_is_star;
    let token_after_is_at = facts.token_after_is(TokenType::At);
    let token_after_is_open_brace = facts.token_after_is(TokenType::OpenBrace);
    let token_after_is_arrow = matches!(
        facts.token_after_type,
        Some(TokenType::Arrow | TokenType::ArrowWide)
    );

    let token_before_is_export = facts.token_before_is_keyword(CommentSeamKeyword::Export);
    let token_before_is_implements = facts.token_before_is_keyword(CommentSeamKeyword::Implements);

    // own-line comments after `implements` should stay on the class declaration seam
    if has_leading_newline
        && token_before_is_implements
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPostfix));
    }

    // inline class and struct head comments before `{` stay on the declaration head.
    if !has_leading_newline
        && comment_is_star
        && token_after_is_open_brace
        && let Some(target_node) = token_before_span
            .zip(token_after_span)
            .and_then(|(before, after)| {
                find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
            })
            .filter(|owner| declaration_owner_has_head_before_open_brace(tree, *owner))
            .or_else(|| {
                left_owner
                    .filter(|owner| declaration_owner_has_head_before_open_brace(tree, *owner))
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments directly before decorators should bind to the right declaration owner
    if token_after_is_at {
        let member_target_from_token = token_after.and_then(|token_after_index| {
            find_next_member_owner_from_token(tree, owner_index, token_after_index)
        });
        let member_target = right_owner.and_then(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
        });
        let declaration_target = token_after
            .and_then(|token_after_index| {
                find_next_declaration_owner_from_token(tree, owner_index, token_after_index)
            })
            .or_else(|| {
                right_owner
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            })
            .or_else(|| {
                left_owner
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            })
            .or_else(|| {
                right_owner.filter(|owner| tree.get_node_type(*owner) == ast::NodeType::Declaration)
            })
            .or_else(|| {
                left_owner.filter(|owner| tree.get_node_type(*owner) == ast::NodeType::Declaration)
            });

        if let Some(target_node) = member_target_from_token
            .or(member_target)
            .or(declaration_target)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            if has_leading_newline {
                return Some((Some(target_node), AnnotationPosition::BlockPrefix));
            }
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    // comments between parameter list and arrow belong to the enclosing arrow declaration seam
    if token_after_is_arrow
        && comment_is_star
        && let (Some(token_before_span), Some(token_after_span)) =
            (token_before_span, token_after_span)
        && let Some(owner) = find_smallest_owner_enclosing_range(
            tree,
            token_before_span.span.start,
            token_after_span.span.end,
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, owner);
        return Some((Some(target_node), AnnotationPosition::BlockInfix));
    }

    // export seam comments belong to the declaration head owner
    if token_before_is_export
        && has_trailing_newline
        && let Some(owner) = token_before_span
            .zip(token_after_span)
            .and_then(|(before, after)| {
                find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, owner);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    None
}
