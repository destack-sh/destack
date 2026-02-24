use ast::{
    AnnotationPosition, Argument, Expression, Keyword, LocalNodeId, NodeParentIndex, NodeTree,
    NodeType, TokenSpan, TokenType,
};
use destack_ast as ast;
use destack_source::{File, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use crate::format::comments::blank::blank_trivia_attachment;
use crate::format::comments::boundary::{
    CommentAttachment, CommentAttachmentOwners, CommentSeamContext, CommentSeamData,
    CommentSeamKeyword, CommentSeamOwnerCache, comment_seam_owner, delimiters_match,
    is_close_delimiter_token, is_open_delimiter_token, previous_non_newline_token_index,
};
use crate::format::comments::declaration::try_attach_comment_declaration;
use crate::format::comments::expression::try_attach_comment_expression;
use crate::format::comments::operator::try_attach_comment_assignment;
use crate::format::comments::ownership::{
    find_owner_at_or_after_token_with_node_type, find_preferred_owner_starting_at,
    find_smallest_owner_enclosing_range, find_smallest_owner_enclosing_token, is_block_like_owner,
    is_trivia_excluded_owner_node_id, lowest_common_owner_ancestor,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_start, promote_owner_to_node_type_ancestor,
};
use crate::format::comments::statement::{
    try_attach_comment_block_body, try_attach_comment_statement_prefix,
    try_attach_comment_statement_suffix,
};
use crate::format::context::{Annotation, FormatterAnnotationEntry};

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
fn formatter_token_neighbor_index(semantic_tokens: &[TokenSpan]) -> FormatterTokenNeighborIndex {
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
fn formatter_owner_start_end_by_token(
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
fn formatter_nearest_owner_start_by_token(
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
fn formatter_nearest_owner_end_by_token(
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
pub(crate) fn formatter_trivia_owner_index(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
) -> FormatterTriviaOwnerIndex {
    let neighbor_index = formatter_token_neighbor_index(semantic_tokens);
    let (owner_start_by_token, owner_end_by_token) =
        formatter_owner_start_end_by_token(tree, semantic_tokens);
    let nearest_owner_start_by_token = formatter_nearest_owner_start_by_token(
        semantic_tokens,
        &owner_start_by_token,
        &neighbor_index,
    );
    let nearest_owner_end_by_token =
        formatter_nearest_owner_end_by_token(semantic_tokens, &owner_end_by_token, &neighbor_index);

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
pub(crate) fn formatter_trivia_seam_index(tree: &NodeTree) -> FormatterTriviaSeamIndex {
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

/// Try to attach one comment inside matching delimiters as container infix trivia.
fn try_attach_comment_delimiter_interior(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
) -> Option<CommentAttachment> {
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
        let expression_id = LocalNodeId::<ast::Expression>::new(container_owner);
        if let ast::Expression::Block(block_id) = tree.get(expression_id) {
            block_id.id
        } else {
            normalize_formatter_trivia_target_owner(tree, container_owner)
        }
    } else {
        normalize_formatter_trivia_target_owner(tree, container_owner)
    };

    Some((Some(target_node), ast::AnnotationPosition::BlockInfix))
}

/// Try to attach one seam comment between parameter name and type to the parameter owner.
fn try_attach_comment_parameter_type_boundary(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachment> {
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
    Some((Some(target_node), ast::AnnotationPosition::BlockInfix))
}

/// Normalize line comments after object member trailing commas inside call arguments.
fn normalize_trailing_object_member_comment_attachment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    attachment: CommentAttachment,
) -> CommentAttachment {
    let (owner, position) = attachment;
    let Some(owner_id) = owner else {
        return (owner, position);
    };

    if !seam.comment_is_line
        || seam.has_leading_newline
        || !seam.token_before_is(TokenType::Comma)
        || !seam.token_after_is(TokenType::CloseBrace)
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
    (
        Some(member_owner),
        ast::AnnotationPosition::LinePostfixBoundary,
    )
}

/// Resolve one comment trivia target owner and position from one token seam.
pub(crate) fn comment_trivia_attachment(
    file: &File,
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    trivia: ast::CommentTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
) -> CommentAttachment {
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
        token_keyword_by_span,
        trivia,
        parents,
        token_before,
        token_after,
        token_before_span,
        token_after_span,
    };
    let owners = CommentAttachmentOwners::new(left_owner, right_owner);
    let seam = CommentSeamData::build(&context);
    let mut seam_owner_cache = CommentSeamOwnerCache::default();

    let attachment = if let Some(attachment) = try_attach_comment_delimiter_interior(tree, &context)
    {
        attachment
    } else if let Some(attachment) =
        try_attach_comment_parameter_type_boundary(tree, parents, &context, owners)
    {
        attachment
    } else if let Some(attachment) = try_attach_comment_expression(
        tree,
        owner_index,
        parents,
        &context,
        &seam,
        &mut seam_owner_cache,
        owners,
    ) {
        attachment
    } else if let Some(attachment) = try_attach_comment_statement_prefix(
        tree,
        parents,
        &context,
        &seam,
        &mut seam_owner_cache,
        owners,
    ) {
        attachment
    } else if let Some(attachment) =
        try_attach_comment_declaration(tree, owner_index, parents, &context, &seam, owners)
    {
        attachment
    } else if let Some(attachment) =
        try_attach_comment_statement_suffix(tree, parents, &seam, owners)
    {
        attachment
    } else if let Some(attachment) = try_attach_comment_assignment(
        tree,
        parents,
        &context,
        &seam,
        &mut seam_owner_cache,
        owners,
    ) {
        attachment
    } else if let Some(attachment) = try_attach_comment_block_body(tree, &seam, owners) {
        attachment
    } else {
        attach_comment_default(&context, &seam, &mut seam_owner_cache, owners)
    };

    normalize_trailing_object_member_comment_attachment(tree, parents, &context, &seam, attachment)
}

/// Build formatter annotation projection for semantic annotations and trivia.
pub(crate) fn formatter_annotation_projection(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
) -> (
    Vec<FormatterAnnotationEntry>,
    Vec<SmallVec<[LocalNodeId<Annotation>; 4]>>,
) {
    let node_count = tree.next_id() as usize;
    let mut entries = Vec::new();
    let mut by_node_id = vec![SmallVec::new(); node_count];
    let mut comment_targets = Vec::<(Span, u32)>::new();
    let owner_index = formatter_trivia_owner_index(tree, tokens);

    // add parser semantic annotations first
    for (&target_id, annotation_ids) in tree.get_all_annotations() {
        if target_id as usize >= by_node_id.len() {
            continue;
        }

        for &annotation_id in annotation_ids {
            let ast_annotation = tree.get(annotation_id);
            let mut target_node_id = target_id;

            // doc comments that sit directly before decorators should bind to the decorated declaration
            if matches!(ast_annotation, ast::Annotation::Doc { .. }) {
                let annotation_span = tree.get_span(annotation_id);
                let mut token_index =
                    tokens.partition_point(|token| token.span.start < annotation_span.end);
                while let Some(token) = tokens.get(token_index).copied() {
                    if matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
                        token_index += 1;
                        continue;
                    }

                    if token.token.ty == TokenType::At
                        && let Some(declaration_id) = find_owner_at_or_after_token_with_node_type(
                            tree,
                            &owner_index,
                            token_index,
                            NodeType::Declaration,
                        )
                    {
                        target_node_id = declaration_id;
                    }

                    break;
                }
            }

            let annotation = match ast_annotation {
                ast::Annotation::Doc { node, position } => Annotation::Doc {
                    node: *node,
                    position: *position,
                },
                ast::Annotation::Decorator { node, position } => Annotation::Decorator {
                    node: *node,
                    position: *position,
                },
            };

            let local_id = LocalNodeId::new(entries.len() as u32);
            entries.push(FormatterAnnotationEntry {
                annotation,
                span: tree.get_span(annotation_id),
            });
            if target_node_id as usize >= by_node_id.len() {
                continue;
            }
            by_node_id[target_node_id as usize].push(local_id);
        }
    }

    // build formatter-side seam indexes for trivia placement
    let seam_index = formatter_trivia_seam_index(tree);

    // add comment trivia with formatter-side placement resolution
    for trivia in tree.comment_trivia().iter().copied() {
        let (target_id, position) = comment_trivia_attachment(
            file,
            tree,
            tokens,
            token_keyword_by_span,
            trivia,
            &owner_index,
            parents,
        );

        let Some(target_id) = target_id else {
            continue;
        };
        if target_id as usize >= by_node_id.len() {
            continue;
        }
        comment_targets.push((trivia.span, target_id));

        let local_id = LocalNodeId::new(entries.len() as u32);
        entries.push(FormatterAnnotationEntry {
            annotation: Annotation::Comment {
                node: trivia.comment,
                position,
            },
            span: trivia.span,
        });
        by_node_id[target_id as usize].push(local_id);
    }

    // add blank trivia with formatter-side placement resolution
    for trivia in tree.blank_trivia().iter().copied() {
        let (mut target_id, mut position) = blank_trivia_attachment(
            tree,
            tokens,
            token_keyword_by_span,
            trivia,
            &owner_index,
            &seam_index,
            parents,
        );

        if target_id.is_none() {
            let next_comment = comment_targets
                .iter()
                .find(|(span, _)| span.start >= trivia.span.end)
                .copied();
            let previous_comment = comment_targets
                .iter()
                .rev()
                .find(|(span, _)| span.end <= trivia.span.start)
                .copied();
            if let (Some((_, previous_comment_target)), Some((_, next_comment_target))) =
                (previous_comment, next_comment)
                && previous_comment_target == next_comment_target
            {
                let comment_target = next_comment_target;
                target_id = Some(comment_target);
                position = ast::AnnotationPosition::BlockPrefix;
            }
        }

        let Some(target_id) = target_id else {
            continue;
        };
        if target_id as usize >= by_node_id.len() {
            continue;
        }
        let local_id = LocalNodeId::new(entries.len() as u32);
        entries.push(FormatterAnnotationEntry {
            annotation: Annotation::Blank {
                node: trivia.blank,
                position,
            },
            span: trivia.span,
        });
        by_node_id[target_id as usize].push(local_id);
    }

    // keep node-local annotation order source-stable
    for annotation_ids in &mut by_node_id {
        if annotation_ids.len() <= 1 {
            continue;
        }

        annotation_ids.sort_by(
            |left: &LocalNodeId<Annotation>, right: &LocalNodeId<Annotation>| {
                let left_span = entries[left.id as usize].span;
                let right_span = entries[right.id as usize].span;
                left_span
                    .start
                    .cmp(&right_span.start)
                    .then(left_span.end.cmp(&right_span.end))
                    .then(left.id.cmp(&right.id))
            },
        );
    }

    (entries, by_node_id)
}

/// Resolve one line comment after ternary `:` by normalizing the left owner seam.
pub(crate) fn attach_line_comment_after_ternary_colon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    left_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    let target_owner = left_owner
        .map(|owner| normalize_owner_with_shared_end(tree, parents, owner, token_before_span))?;
    if tree.get_node_type(target_owner) != NodeType::Expression {
        return None;
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Promote one owner from a tree tag path to the enclosing tree expression when needed.
pub(crate) fn promote_owner_to_tree_expression_parent(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> u32 {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return owner_id;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    if !matches!(tree.get(expression_id), Expression::Path { .. }) {
        return owner_id;
    }

    let Some(parent_id) = parents.get_by_id(owner_id) else {
        return owner_id;
    };
    if tree.get_node_type(parent_id) != NodeType::Expression {
        return owner_id;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    if matches!(
        tree.get(parent_expression_id),
        Expression::TreeExpression { .. }
    ) {
        return parent_id;
    }

    owner_id
}

/// Resolve one line comment after `,` and before `}` by attaching to the enclosing property owner.
pub(crate) fn attach_trailing_comma_close_brace_property_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    token_before_span: Option<TokenSpan>,
    left_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let target_owner = token_before_span.and_then(|comma_token| {
        let search_start = comma_token.span.start.saturating_sub(1);
        (search_start < comma_token.span.start).then(|| {
            find_smallest_owner_enclosing_range(tree, search_start, comma_token.span.start)
        })?
    });
    let target_owner = target_owner.or(left_owner)?;

    let target_owner = normalize_owner_with_shared_end(
        tree,
        parents,
        target_owner,
        token_before_span.map(|token| token.span),
    );
    let property_owner =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Property)?;
    let property_owner = normalize_formatter_trivia_target_owner(tree, property_owner);
    Some((
        Some(property_owner),
        AnnotationPosition::LinePostfixBoundary,
    ))
}

/// Resolve one star comment before `:` by attaching to tree-expression right or left boundary owner.
pub(crate) fn attach_star_comment_before_ternary_colon(
    tree: &NodeTree,
    left_owner: Option<u32>,
    right_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let left_target_owner =
        left_owner.filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)?;

    let right_prefers_tree_expression = right_owner.is_some_and(|owner| {
        tree.get_node_type(owner) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(owner)),
                Expression::TreeExpression { .. }
            )
    });
    if right_prefers_tree_expression && let Some(target_owner) = right_owner {
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    let target_expression = LocalNodeId::<Expression>::new(left_target_owner);
    if let Expression::Parenthesized { expression } = tree.get(target_expression) {
        let target_owner = normalize_formatter_trivia_target_owner(tree, expression.id);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, left_target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve the default comment trivia rules after specialized seam cases.
pub(crate) fn attach_comment_default(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> CommentAttachment {
    let tree = context.tree;

    // own line cases
    if let Some(attachment) = try_attach_own_line_comment(context, seam, seam_owner_cache, owners) {
        return attachment;
    }

    // inline multiline block comments before line end
    if seam.comment_is_multiline_star && seam.has_trailing_newline && !seam.has_leading_newline {
        let parents: &NodeParentIndex = context.parents;
        let token_before_span = context.token_before_span.map(|token| token.span);
        if let Some(left_owner) = owners.left {
            let target_node =
                normalize_owner_with_shared_end(tree, parents, left_owner, token_before_span);
            return (Some(target_node), AnnotationPosition::BlockPostfix);
        }
    }

    // trailing line and terminal cases
    if let Some(attachment) =
        try_attach_trailing_line_comment(context, seam, seam_owner_cache, owners)
    {
        return attachment;
    }

    // same line default prefers right, then left
    if let Some(attachment) = try_attach_same_line_default(context, seam, owners) {
        return attachment;
    }

    // comment only files can still anchor to one enclosing owner
    if context.token_before.is_none() && context.token_after.is_none() {
        if let Some(target_node) = find_smallest_owner_enclosing_range(
            tree,
            context.trivia.span.start,
            context.trivia.span.end,
        ) {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }
    }

    (None, AnnotationPosition::BlockInfix)
}

/// Return whether one owner is an if expression nested in another if expression.
fn owner_is_nested_if_expression(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    if !matches!(
        tree.get(LocalNodeId::<Expression>::new(owner_id)),
        Expression::If { .. }
    ) {
        return false;
    }

    let Some(parent_id) = parents.get_by_id(owner_id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != NodeType::Expression {
        return false;
    }

    matches!(
        tree.get(LocalNodeId::<Expression>::new(parent_id)),
        Expression::If { .. }
    )
}

/// Attach own line comments with dedicated own line rules.
fn try_attach_own_line_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline {
        return None;
    }

    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let left_owner = owners.left.or_else(|| {
        context
            .token_before
            .and_then(|token_index| {
                previous_non_newline_token_index(context.semantic_tokens, token_index)
            })
            .and_then(|token_index| context.semantic_tokens.get(token_index))
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
    });
    let right_owner = owners.right;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let seam_owner = comment_seam_owner(context, seam_owner_cache);

    // own line comments before nested else branches should stay on the if seam
    if seam.token_after_is_keyword(CommentSeamKeyword::Else) {
        let seam_owner_is_nested_if =
            seam_owner.is_some_and(|owner| owner_is_nested_if_expression(tree, parents, owner));
        let right_owner_is_block =
            right_owner.is_some_and(|owner| is_block_like_owner(tree, owner));

        if seam_owner_is_nested_if
            && !right_owner_is_block
            && let Some(target_node) = seam_owner
        {
            let target_node =
                normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockPostfix));
        }

        if let Some(mut target_node) = right_owner.or(seam_owner).or(left_owner) {
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

            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            let position = if !seam.comment_is_line && !seam.comment_is_multiline_star {
                AnnotationPosition::LinePrefix
            } else {
                AnnotationPosition::BlockPrefix
            };
            return Some((Some(target_node), position));
        }
    }

    // own line comments inside parenthesized groups before < should stay with the right side
    let token_before_is_open_delimiter =
        seam.token_before_type.is_some_and(is_open_delimiter_token);
    if token_before_is_open_delimiter
        && seam.token_after_is(ast::TokenType::LessThan)
        && let Some(target_node) = right_owner.or(seam_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // own line comments after member semicolons should stay on the enclosing member
    if seam.token_after_is(ast::TokenType::Semicolon)
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        if let Some(member_owner) =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Member)
        {
            let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
            return Some((Some(member_owner), AnnotationPosition::BlockPostfix));
        }
    }

    // own line comments before separators and closers belong to the left owner
    if seam.token_after_prefers_left
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        let position = if seam.comment_is_line {
            if seam.token_after_is(ast::TokenType::CloseBrace) {
                AnnotationPosition::BlockPostfix
            } else {
                AnnotationPosition::LinePostfixBoundary
            }
        } else {
            AnnotationPosition::BlockPostfix
        };
        return Some((Some(target_node), position));
    }

    // default own line comment binding: right owner
    if let Some(target_node) = right_owner {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    None
}

/// Attach trailing line comments and terminal seam comments.
fn try_attach_trailing_line_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    seam_owner_cache: &mut CommentSeamOwnerCache,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachment> {
    if !seam.has_trailing_newline && context.token_after.is_some() {
        return None;
    }

    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let left_owner = owners.left;
    let right_owner = owners.right;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let seam_owner = comment_seam_owner(context, seam_owner_cache);
    let is_same_line_line_comment = seam.comment_is_line && !seam.has_leading_newline;

    // trailing comments after empty if statements stay on the empty consequent
    if is_same_line_line_comment && seam.token_before_is(ast::TokenType::Semicolon) {
        let left_if_owner = left_owner
            .map(|owner| normalize_owner_with_shared_end(tree, parents, owner, token_before_span));
        for candidate_owner in [left_if_owner, seam_owner].into_iter().flatten() {
            if let Some(then_owner) = if_expression_then_owner_without_else(tree, candidate_owner) {
                let attachment = (Some(then_owner), AnnotationPosition::LinePostfixBoundary);
                return Some(attachment);
            }
        }
    }

    // line comments after trailing commas before `}` should stay on the enclosing member
    if is_same_line_line_comment
        && seam.token_before_is(ast::TokenType::Comma)
        && seam.token_after_is(ast::TokenType::CloseBrace)
        && let Some(attachment) = attach_trailing_comma_close_brace_property_line_comment(
            tree,
            parents,
            context.token_before_span,
            left_owner,
        )
    {
        return Some(attachment);
    }

    // trailing comments after inline break and continue statements stay on the control statement
    if is_same_line_line_comment
        && let Some(token_before_span) = context.token_before_span
        && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token_before_span.span)
        && tree.get_node_type(target_node) == NodeType::Expression
    {
        let target_expression = LocalNodeId::<Expression>::new(target_node);
        if matches!(
            tree.get(target_expression),
            Expression::Break { .. } | Expression::Continue { .. }
        ) {
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // line comments after ternary `:` stay with the consequent branch boundary
    if is_same_line_line_comment
        && seam.token_before_is(ast::TokenType::Colon)
        && !seam.token_before_is_return_type_colon
        && let Some(attachment) =
            attach_line_comment_after_ternary_colon(tree, parents, left_owner, token_before_span)
    {
        return Some(attachment);
    }

    // block comments immediately before ternary `:` stay on the branch seam
    if seam.comment_is_star
        && seam.token_after_is(ast::TokenType::Colon)
        && !seam.token_before_is_return_type_colon
        && let Some(attachment) =
            attach_star_comment_before_ternary_colon(tree, left_owner, right_owner)
    {
        return Some(attachment);
    }

    // line comments before tree-expression container `}` stay on the container value expression
    if is_same_line_line_comment
        && seam.token_after_is(ast::TokenType::CloseBrace)
        && let Some(left_expression_owner) = left_owner
            .filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)
            .filter(|owner| {
                parents
                    .get_by_id(*owner)
                    .is_some_and(|parent_id| tree.get_node_type(parent_id) == NodeType::Argument)
            })
    {
        return Some((
            Some(left_expression_owner),
            AnnotationPosition::LinePostfixBoundary,
        ));
    }

    if is_same_line_line_comment
        && seam.token_after_is(ast::TokenType::CloseBrace)
        && let Some(target_owner) = left_owner
            .filter(|owner| tree.get_node_type(*owner) == NodeType::Argument)
            .or_else(|| seam_owner.filter(|owner| tree.get_node_type(*owner) == NodeType::Argument))
    {
        let argument_id = LocalNodeId::<Argument>::new(target_owner);
        let value_id = match tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => *value,
        };
        let attachment = (Some(value_id.id), AnnotationPosition::LinePostfixBoundary);
        return Some(attachment);
    }

    // comments after most : seams stay with the left segment
    let colon_prefers_left = seam.token_before_is(ast::TokenType::Colon)
        && !seam.token_before_is_return_type_colon
        && seam.comment_is_line;

    if seam.seam_binds_right
        && !seam.token_after_prefers_left
        && !colon_prefers_left
        && let Some(target_owner) = right_owner
    {
        let target_owner = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_owner, span.start))
            .unwrap_or(target_owner);
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    // comments after one member semicolon belong to the member, not the value expression
    if seam.token_before_is(ast::TokenType::Semicolon)
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        if let Some(member_owner) =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Member)
        {
            let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
            return Some((Some(member_owner), AnnotationPosition::LinePostfixBoundary));
        }
    }

    if let Some(target_node) = left_owner {
        let should_keep_terminal_left_owner = seam.token_after_is(ast::TokenType::CloseParenthesis)
            && seam.token_before_is(ast::TokenType::Literal)
            && !seam.token_before_is(ast::TokenType::Comma);
        let target_node = if should_keep_terminal_left_owner {
            target_node
        } else {
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span)
        };
        let position = if should_keep_terminal_left_owner {
            AnnotationPosition::LinePostfix
        } else {
            AnnotationPosition::LinePostfixBoundary
        };
        return Some((Some(target_node), position));
    }

    None
}

/// Return the then-branch owner id for one if-expression owner that has no else branch.
pub(crate) fn if_expression_then_owner_without_else(tree: &NodeTree, owner: u32) -> Option<u32> {
    if tree.get_node_type(owner) != NodeType::Expression {
        return None;
    }

    let seam_expression = LocalNodeId::<Expression>::new(owner);
    if let Expression::If {
        then_expression,
        else_expression,
        ..
    } = tree.get(seam_expression)
        && else_expression.is_none()
    {
        return Some(then_expression.id);
    }

    None
}

/// Attach same line comments when trailing rules do not apply.
fn try_attach_same_line_default(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentOwners,
) -> Option<CommentAttachment> {
    let tree = context.tree;
    let parents: &NodeParentIndex = context.parents;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let token_after_span = context.token_after_span.map(|token| token.span);
    let left_owner = owners.left;
    let right_owner = owners.right;

    // same line seams that precede separators and operators prefer left postfix
    if seam.token_after_prefers_left
        && let Some(target_node) = left_owner
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // same line seams prefer the right owner as line prefix
    if let Some(target_node) = right_owner {
        let target_node = token_after_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_node, span.start))
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // default to left owner as line postfix
    if let Some(target_node) = left_owner {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    None
}
