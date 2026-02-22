use ast::{NodeTree, NodeType, TokenSpan, TokenType};
use destack_ast as ast;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::format::comments::owner::is_trivia_excluded_owner_node_id;

const NO_TOKEN_INDEX: u32 = u32::MAX;

struct FormatterTokenNeighborIndex {
    previous_attachable: Vec<Option<usize>>,
    next_attachable: Vec<Option<usize>>,
}

#[derive(Debug)]
pub(crate) struct FormatterTriviaOwnerIndex {
    pub(crate) owner_start_by_token: Vec<Option<u32>>,
    pub(crate) owner_end_by_token: Vec<Option<u32>>,
    pub(crate) nearest_owner_start_by_token: Vec<Option<u32>>,
    pub(crate) nearest_owner_end_by_token: Vec<Option<u32>>,
}

#[derive(Debug)]
pub(crate) struct FormatterTriviaSeamIndex {
    pub(crate) comment_seams: FxHashSet<u64>,
    pub(crate) line_comment_seams: FxHashSet<u64>,
    pub(crate) first_comment_start_by_seam: FxHashMap<u64, u32>,
}

/// Return true when one semantic token can own trivia seams.
fn is_attachable_semantic_token_for_trivia(token_type: TokenType) -> bool {
    !matches!(token_type, TokenType::Newline | TokenType::End)
}

/// Build previous and next attachable semantic token indexes.
fn build_formatter_token_neighbor_index(
    semantic_tokens: &[TokenSpan],
) -> FormatterTokenNeighborIndex {
    let mut previous_attachable = vec![None; semantic_tokens.len()];
    let mut next_attachable = vec![None; semantic_tokens.len()];

    // previous attachable token before each semantic token
    let mut previous = None;
    for (index, token) in semantic_tokens.iter().enumerate() {
        previous_attachable[index] = previous;
        if is_attachable_semantic_token_for_trivia(token.token.ty) {
            previous = Some(index);
        }
    }

    // next attachable token after each semantic token
    let mut next = None;
    for index in (0..semantic_tokens.len()).rev() {
        next_attachable[index] = next;
        if is_attachable_semantic_token_for_trivia(semantic_tokens[index].token.ty) {
            next = Some(index);
        }
    }

    FormatterTokenNeighborIndex {
        previous_attachable,
        next_attachable,
    }
}

/// Update one owner slot when one candidate outranks the current owner.
fn update_best_formatter_owner_slot(
    owner_by_token: &mut [Option<u32>],
    owner_length_by_token: &mut [u32],
    owner_kind_rank_by_token: &mut [u8],
    token_index: usize,
    node_id: u32,
    length: u32,
    node_kind_rank: u8,
) {
    let best_length = owner_length_by_token[token_index];
    let best_kind_rank = owner_kind_rank_by_token[token_index];
    let best_owner = owner_by_token[token_index];
    let should_replace = length < best_length
        || (length == best_length && node_kind_rank < best_kind_rank)
        || (length == best_length
            && node_kind_rank == best_kind_rank
            && best_owner.is_none_or(|best| node_id < best));
    if should_replace {
        owner_length_by_token[token_index] = length;
        owner_kind_rank_by_token[token_index] = node_kind_rank;
        owner_by_token[token_index] = Some(node_id);
    }
}

/// Build best start and end owner ids for attachable semantic token indexes.
fn build_formatter_owner_start_end_by_token(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
) -> (Vec<Option<u32>>, Vec<Option<u32>>) {
    let mut start_token_by_offset = FxHashMap::<u32, usize>::default();
    start_token_by_offset.reserve(semantic_tokens.len());
    let mut end_token_by_offset = FxHashMap::<u32, usize>::default();
    end_token_by_offset.reserve(semantic_tokens.len());
    for (index, token) in semantic_tokens.iter().copied().enumerate() {
        if !is_attachable_semantic_token_for_trivia(token.token.ty) {
            continue;
        }

        start_token_by_offset.insert(token.span.start, index);
        end_token_by_offset.insert(token.span.end, index);
    }

    let mut owner_start_by_token = vec![None; semantic_tokens.len()];
    let mut owner_end_by_token = vec![None; semantic_tokens.len()];
    let mut owner_start_length_by_token = vec![u32::MAX; semantic_tokens.len()];
    let mut owner_end_length_by_token = vec![u32::MAX; semantic_tokens.len()];
    let mut owner_start_kind_rank_by_token = vec![u8::MAX; semantic_tokens.len()];
    let mut owner_end_kind_rank_by_token = vec![u8::MAX; semantic_tokens.len()];

    // choose smallest owner for one token seam and share one node scan for start and end
    let mut node_id = 0u32;
    while node_id < tree.next_id() {
        if is_trivia_excluded_owner_node_id(tree, node_id) {
            node_id += 1;
            continue;
        }

        let span = tree.get_span_by_id(node_id);
        let length = span.end.saturating_sub(span.start);
        let node_kind_rank = if tree.get_node_type(node_id) == NodeType::Expression {
            1
        } else {
            0
        };

        if let Some(token_index) = start_token_by_offset.get(&span.start).copied() {
            update_best_formatter_owner_slot(
                &mut owner_start_by_token,
                &mut owner_start_length_by_token,
                &mut owner_start_kind_rank_by_token,
                token_index,
                node_id,
                length,
                node_kind_rank,
            );
        }

        if let Some(token_index) = end_token_by_offset.get(&span.end).copied() {
            update_best_formatter_owner_slot(
                &mut owner_end_by_token,
                &mut owner_end_length_by_token,
                &mut owner_end_kind_rank_by_token,
                token_index,
                node_id,
                length,
                node_kind_rank,
            );
        }

        node_id += 1;
    }

    (owner_start_by_token, owner_end_by_token)
}

/// Build nearest start-owner ids for each semantic token index.
fn build_formatter_nearest_owner_start_by_token(
    semantic_tokens: &[TokenSpan],
    owner_start_by_token: &[Option<u32>],
    neighbor_index: &FormatterTokenNeighborIndex,
) -> Vec<Option<u32>> {
    let mut nearest_owner_start_by_token = vec![None; semantic_tokens.len()];

    for index in (0..semantic_tokens.len()).rev() {
        let token = semantic_tokens[index];
        if !is_attachable_semantic_token_for_trivia(token.token.ty) {
            nearest_owner_start_by_token[index] = neighbor_index.next_attachable[index]
                .and_then(|next_index| nearest_owner_start_by_token[next_index]);
            continue;
        }

        nearest_owner_start_by_token[index] = owner_start_by_token[index].or_else(|| {
            neighbor_index.next_attachable[index]
                .and_then(|next_index| nearest_owner_start_by_token[next_index])
        });
    }

    nearest_owner_start_by_token
}

/// Build nearest end-owner ids for each semantic token index.
fn build_formatter_nearest_owner_end_by_token(
    semantic_tokens: &[TokenSpan],
    owner_end_by_token: &[Option<u32>],
    neighbor_index: &FormatterTokenNeighborIndex,
) -> Vec<Option<u32>> {
    let mut nearest_owner_end_by_token = vec![None; semantic_tokens.len()];

    for index in 0..semantic_tokens.len() {
        let token = semantic_tokens[index];
        if !is_attachable_semantic_token_for_trivia(token.token.ty) {
            nearest_owner_end_by_token[index] = neighbor_index.previous_attachable[index]
                .and_then(|previous_index| nearest_owner_end_by_token[previous_index]);
            continue;
        }

        nearest_owner_end_by_token[index] = owner_end_by_token[index].or_else(|| {
            neighbor_index.previous_attachable[index]
                .and_then(|previous_index| nearest_owner_end_by_token[previous_index])
        });
    }

    nearest_owner_end_by_token
}

/// Build owner indexes for formatter-side trivia attachment.
pub(crate) fn build_formatter_trivia_owner_index(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
) -> FormatterTriviaOwnerIndex {
    let neighbor_index = build_formatter_token_neighbor_index(semantic_tokens);
    let (owner_start_by_token, owner_end_by_token) =
        build_formatter_owner_start_end_by_token(tree, semantic_tokens);
    let nearest_owner_start_by_token = build_formatter_nearest_owner_start_by_token(
        semantic_tokens,
        &owner_start_by_token,
        &neighbor_index,
    );
    let nearest_owner_end_by_token = build_formatter_nearest_owner_end_by_token(
        semantic_tokens,
        &owner_end_by_token,
        &neighbor_index,
    );

    FormatterTriviaOwnerIndex {
        owner_start_by_token,
        owner_end_by_token,
        nearest_owner_start_by_token,
        nearest_owner_end_by_token,
    }
}

/// Encode one trivia token seam into one compact key.
#[inline]
pub(crate) fn encode_trivia_seam(token_before: u32, token_after: u32) -> u64 {
    ((token_before as u64) << 32) | token_after as u64
}

/// Build formatter-side trivia seam indexes.
pub(crate) fn build_formatter_trivia_seam_index(tree: &NodeTree) -> FormatterTriviaSeamIndex {
    let mut comment_seams = FxHashSet::default();
    comment_seams.reserve(tree.comment_trivia().len());
    let mut line_comment_seams = FxHashSet::default();
    line_comment_seams.reserve(tree.comment_trivia().len());
    let mut first_comment_start_by_seam = FxHashMap::default();
    first_comment_start_by_seam.reserve(tree.comment_trivia().len());

    // cache comment seam keys once for blank attachment checks
    for trivia in tree.comment_trivia().iter().copied() {
        let seam = encode_trivia_seam(trivia.boundary.token_before, trivia.boundary.token_after);
        comment_seams.insert(seam);
        first_comment_start_by_seam
            .entry(seam)
            .and_modify(|start: &mut u32| *start = (*start).min(trivia.span.start))
            .or_insert(trivia.span.start);

        if tree.get(trivia.comment).style == ast::CommentStyle::Slash {
            line_comment_seams.insert(seam);
        }
    }

    FormatterTriviaSeamIndex {
        comment_seams,
        line_comment_seams,
        first_comment_start_by_seam,
    }
}

/// Decode one compact token index with sentinel for none.
pub(crate) fn decode_token_index(token_index: u32) -> Option<usize> {
    (token_index != NO_TOKEN_INDEX).then_some(token_index as usize)
}
