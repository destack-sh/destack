use ast::{NodeParentIndex, NodeTree, TokenSpan};
use destack_ast as ast;
use destack_source::File;

use super::index::{FormatterTriviaOwnerIndex, decode_token_index};
use super::owner::{find_preferred_owner_starting_at, find_smallest_owner_enclosing_token};
use super::seam::{CommentAttachmentOwners, CommentSeamContext};

/// Initial seam context and owner candidates for one comment attachment decision.
#[derive(Clone, Copy)]
pub(super) struct CommentAttachmentSetup<'a> {
    /// The seam context used across attachment rules.
    pub(super) context: CommentSeamContext<'a>,
    /// The nearest left and right owner candidates.
    pub(super) owners: CommentAttachmentOwners,
}

/// Build seam context and initial owner candidates.
pub(super) fn build_comment_attachment_setup<'a>(
    file: &'a File,
    tree: &'a NodeTree,
    semantic_tokens: &'a [TokenSpan],
    trivia: ast::CommentTrivia,
    owner_index: &'a FormatterTriviaOwnerIndex,
    parents: &'a NodeParentIndex,
) -> CommentAttachmentSetup<'a> {
    let token_before = decode_token_index(trivia.boundary.token_before);
    let token_after = decode_token_index(trivia.boundary.token_after);

    let token_before_span = token_before
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let token_after_span = token_after
        .and_then(|index| semantic_tokens.get(index))
        .copied();

    let mut right_owner = token_after
        .and_then(|index| {
            owner_index
                .owner_start_by_token
                .get(index)
                .and_then(|owner| *owner)
        })
        .or_else(|| {
            token_after.and_then(|index| {
                owner_index
                    .nearest_owner_start_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
        });
    let mut left_owner = token_before
        .and_then(|index| {
            owner_index
                .owner_end_by_token
                .get(index)
                .and_then(|owner| *owner)
        })
        .or_else(|| {
            token_before.and_then(|index| {
                owner_index
                    .nearest_owner_end_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
        });

    if right_owner.is_none()
        && let Some(token_after_span) = token_after_span
    {
        right_owner = find_preferred_owner_starting_at(tree, token_after_span.span)
            .or_else(|| find_smallest_owner_enclosing_token(tree, token_after_span.span));
    }

    if left_owner.is_none()
        && let Some(token_before_span) = token_before_span
    {
        left_owner = find_smallest_owner_enclosing_token(tree, token_before_span.span);
    }

    let context = CommentSeamContext {
        file,
        tree,
        semantic_tokens,
        trivia,
        parents,
        token_before,
        token_after,
        token_before_span,
        token_after_span,
    };

    CommentAttachmentSetup {
        context,
        owners: CommentAttachmentOwners::new(left_owner, right_owner),
    }
}
