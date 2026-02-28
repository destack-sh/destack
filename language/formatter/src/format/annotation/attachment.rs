use ast::{
    AnnotationPosition, Block, BlockFormat, CommentDirective, DependencyMode, Expression, Keyword,
    LocalNodeId, NodeParentIndex, NodeTree, NodeType, TokenSpan, TokenType,
};
use destack_ast as ast;
use destack_source::{File, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use super::blank::blank_trivia_attachment;
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, comment_enclosing_owner, delimiters_match, is_close_delimiter_token,
    is_open_delimiter_token,
};
use super::declaration::try_attach_comment_declaration;
use super::endofline::attach_end_of_line_comment;
use super::expression::try_attach_comment_expression;
use super::facts::previous_non_trivia_token_index;
use super::operator::try_attach_comment_assignment;
use super::ownership::{
    find_owner_at_or_after_token, find_owner_at_or_after_token_with_node_type,
    find_preferred_owner_starting_at, find_smallest_owner_enclosing_range,
    find_smallest_owner_enclosing_token, is_trivia_excluded_owner_node_id,
    lowest_common_owner_ancestor, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_by_shared_start,
    promote_owner_to_declaration_ancestor, promote_owner_to_node_type_ancestor,
    promote_rhs_expression_owner,
};
use super::ownline::attach_own_line_comment;
use super::placement::{CommentPlacement, classify_comment_placement};
use super::remaining::attach_remaining_comment;
use super::statement::{
    try_attach_comment_block_body, try_attach_comment_statement_prefix,
    try_attach_comment_statement_suffix,
};
use crate::format::context::{Annotation, FormatterAnnotationEntry};

const NO_TOKEN_INDEX: u32 = u32::MAX;

/// Return whether one comment trivia directive is an ignore marker.
#[inline]
fn comment_directive_is_ignore(directive: CommentDirective) -> bool {
    matches!(
        directive,
        CommentDirective::FormatIgnore
            | CommentDirective::FormatIgnoreFile
            | CommentDirective::FormatIgnoreStart
            | CommentDirective::FormatIgnoreEnd
    )
}

/// Return rhs owner for one doc annotation that belongs to an assignment seam.
fn assignment_like_rhs_owner_for_doc_annotation(
    file: &File,
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(u32, AnnotationPosition)> {
    let annotation_token_index =
        tokens.partition_point(|token| token.span.start < annotation_span.start);
    if annotation_token_index >= tokens.len() {
        return None;
    }

    let previous_index = previous_non_trivia_token_index(tokens, annotation_token_index)?;
    let previous_token_type = tokens[previous_index].token.ty;
    if previous_token_type != TokenType::Assign {
        return None;
    }
    let previous_token_span = tokens[previous_index].span;

    let mut token_after_annotation_index =
        tokens.partition_point(|token| token.span.start < annotation_span.end);
    while let Some(token) = tokens.get(token_after_annotation_index) {
        if matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
            token_after_annotation_index += 1;
            continue;
        }
        break;
    }
    let token_after_span = tokens
        .get(token_after_annotation_index)
        .map(|token| token.span);
    if token_after_span.is_some_and(|span| annotation_span.start >= span.start) {
        return None;
    }

    let assignment_owner =
        find_smallest_owner_enclosing_token(tree, previous_token_span).and_then(|owner_id| {
            let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
                Some(owner_id)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
            };
            if let Some(expression_owner) = expression_owner {
                let expression_id = LocalNodeId::<Expression>::new(expression_owner);
                if let Expression::Assign { right, .. } = tree.get(expression_id) {
                    return Some(right.id);
                }
            }

            let declaration_owner = if tree.get_node_type(owner_id) == NodeType::Declaration {
                Some(owner_id)
            } else {
                promote_owner_to_declaration_ancestor(tree, parents, owner_id)
            }?;
            let declaration_id = LocalNodeId::<ast::Declaration>::new(declaration_owner);
            match tree.get(declaration_id) {
                ast::Declaration::Type { value, .. } => Some(value.id),
                _ => None,
            }
        });

    let fallback_expression_owner = token_after_span
        .and_then(|span| find_preferred_owner_starting_at(tree, span))
        .and_then(|owner_id| {
            if tree.get_node_type(owner_id) == NodeType::Expression {
                Some(owner_id)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
            }
        })
        .or_else(|| {
            find_owner_at_or_after_token_with_node_type(
                tree,
                owner_index,
                token_after_annotation_index,
                NodeType::Expression,
            )
        });

    let expression_owner = assignment_owner.or(fallback_expression_owner)?;
    let expression_owner =
        promote_rhs_expression_owner(tree, parents, expression_owner, token_after_span);

    let annotation_starts_on_assignment_line =
        file.is_same_line(previous_token_span.start, annotation_span.start);
    let annotation_has_newline = annotation_span.start < annotation_span.end
        && !file.is_same_line(annotation_span.start, annotation_span.end.saturating_sub(1));
    let position = if annotation_starts_on_assignment_line && !annotation_has_newline {
        AnnotationPosition::LinePrefix
    } else {
        AnnotationPosition::BlockPrefix
    };

    Some((expression_owner, position))
}

/// Return trailing statement owner for one doc annotation after terminal semicolon.
fn trailing_statement_owner_for_doc_annotation(
    file: &File,
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(u32, AnnotationPosition)> {
    let annotation_token_index =
        tokens.partition_point(|token| token.span.start < annotation_span.start);
    if annotation_token_index >= tokens.len() {
        return None;
    }

    let previous_index = previous_non_trivia_token_index(tokens, annotation_token_index)?;
    if tokens[previous_index].token.ty != TokenType::Semicolon {
        return None;
    }

    let token_after_annotation_index =
        tokens.partition_point(|token| token.span.start < annotation_span.end);
    let has_following_owner =
        find_owner_at_or_after_token(tree, tokens, token_after_annotation_index).is_some()
            || find_owner_at_or_after_token_with_node_type(
                tree,
                owner_index,
                token_after_annotation_index,
                NodeType::Declaration,
            )
            .is_some();
    if has_following_owner {
        return None;
    }

    let semicolon_span = tokens[previous_index].span;
    let owner = find_smallest_owner_enclosing_token(tree, semicolon_span)?;
    let owner = normalize_owner_with_shared_end(tree, parents, owner, Some(semicolon_span));
    let is_same_line =
        file.is_same_line(semicolon_span.end.saturating_sub(1), annotation_span.start);
    let position = if is_same_line {
        AnnotationPosition::LinePostfixBoundary
    } else {
        AnnotationPosition::BlockPostfix
    };

    Some((owner, position))
}

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

    let container_owner = find_preferred_owner_starting_at(tree, token_before_span.span)
        .filter(|owner_id| tree.get_span_by_id(*owner_id).end >= token_after_span.span.end)
        .or_else(|| {
            find_smallest_owner_enclosing_range(
                tree,
                token_before_span.span.start,
                token_after_span.span.end,
            )
        })?;

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
    owners: CommentAttachmentNeighbors,
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
            .preceding
            .zip(owners.following)
            .and_then(|(preceding_owner, following_owner)| {
                lowest_common_owner_ancestor(tree, parents, preceding_owner, following_owner)
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

    let mut following_owner = token_after
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
    let mut preceding_owner = token_before
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

    if following_owner.is_none()
        && let Some(token_after_span) = token_after_span
    {
        following_owner = find_preferred_owner_starting_at(tree, token_after_span.span)
            .or_else(|| find_smallest_owner_enclosing_token(tree, token_after_span.span));
    }

    if preceding_owner.is_none()
        && let Some(token_before_span) = token_before_span
    {
        preceding_owner = find_smallest_owner_enclosing_token(tree, token_before_span.span);
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
    let owners = CommentAttachmentNeighbors::new(preceding_owner, following_owner);
    let seam = CommentSeamData::build(&context);
    let mut enclosing_owner_cache = CommentEnclosingOwnerCache::default();

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
        &mut enclosing_owner_cache,
        owners,
    ) {
        attachment
    } else if let Some(attachment) = try_attach_comment_statement_prefix(
        tree,
        parents,
        &context,
        &seam,
        &mut enclosing_owner_cache,
        owners,
    ) {
        attachment
    } else if let Some(attachment) =
        try_attach_comment_declaration(tree, owner_index, parents, &context, &seam, owners)
    {
        attachment
    } else if let Some(attachment) =
        try_attach_comment_statement_suffix(tree, parents, &context, &seam, owners)
    {
        attachment
    } else if let Some(attachment) = try_attach_comment_assignment(
        tree,
        parents,
        &context,
        &seam,
        &mut enclosing_owner_cache,
        owners,
    ) {
        attachment
    } else if let Some(attachment) = try_attach_comment_block_body(tree, &seam, owners) {
        attachment
    } else {
        attach_comment_default(&context, &seam, &mut enclosing_owner_cache, owners)
    };

    let (target_node, mut position) = attachment;
    if seam.comment_is_line
        && position == AnnotationPosition::BlockPrefix
        && comment_directive_is_ignore(trivia.directive)
    {
        position = AnnotationPosition::LinePrefix;
    }
    let attachment = (target_node, position);

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

    let declaration_is_default_export_value = |target_node_id: u32| {
        if tree.get_node_type(target_node_id) != NodeType::Declaration {
            return false;
        }

        let Some(declaration_expression_node_id) = parents.get_by_id(target_node_id) else {
            return false;
        };
        if tree.get_node_type(declaration_expression_node_id) != NodeType::Expression {
            return false;
        }

        let declaration_expression_id =
            LocalNodeId::<Expression>::new(declaration_expression_node_id);
        let Expression::Declaration(declaration_id) = tree.get(declaration_expression_id) else {
            return false;
        };
        if declaration_id.id != target_node_id {
            return false;
        }

        let mut ancestor_id = parents.get_by_id(declaration_expression_node_id);
        while let Some(node_id) = ancestor_id {
            if tree.get_node_type(node_id) == NodeType::Expression {
                let expression_id = LocalNodeId::<Expression>::new(node_id);
                if let Expression::Export { items, .. } = tree.get(expression_id) {
                    return items.iter().any(|item_id| {
                        let item = tree.get(*item_id);
                        item.mode == DependencyMode::Default
                            && item
                                .value
                                .is_some_and(|value_id| value_id.id == declaration_expression_id.id)
                    });
                }
            }

            ancestor_id = parents.get_by_id(node_id);
        }

        false
    };

    // add parser semantic annotations first
    for (&target_id, annotation_ids) in tree.get_all_annotations() {
        if target_id as usize >= by_node_id.len() {
            continue;
        }

        for &annotation_id in annotation_ids {
            let ast_annotation = tree.get(annotation_id);
            let annotation_span = tree.get_span(annotation_id);
            let mut target_node_id = target_id;
            let mut doc_position_override = None;

            // doc comments that sit directly before decorators should bind to the decorated declaration
            if matches!(ast_annotation, ast::Annotation::Doc { .. }) {
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

                if let Some((trailing_owner, trailing_position)) =
                    trailing_statement_owner_for_doc_annotation(
                        file,
                        tree,
                        parents,
                        &owner_index,
                        tokens,
                        annotation_span,
                    )
                {
                    target_node_id = trailing_owner;
                    doc_position_override = Some(trailing_position);
                } else if let Some((rhs_owner, rhs_position)) =
                    assignment_like_rhs_owner_for_doc_annotation(
                        file,
                        tree,
                        parents,
                        &owner_index,
                        tokens,
                        annotation_span,
                    )
                {
                    target_node_id = rhs_owner;
                    doc_position_override = Some(rhs_position);
                }
            }

            // decorators should stay attached to declarations and members,
            // not intermediary expression wrappers
            if matches!(ast_annotation, ast::Annotation::Decorator { .. })
                && tree.get_node_type(target_node_id) == NodeType::Expression
            {
                let mut token_index =
                    tokens.partition_point(|token| token.span.start < annotation_span.end);
                while let Some(token) = tokens.get(token_index).copied() {
                    if matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
                        token_index += 1;
                        continue;
                    }

                    if let Some(member_target) = find_owner_at_or_after_token_with_node_type(
                        tree,
                        &owner_index,
                        token_index,
                        NodeType::Member,
                    )
                    .and_then(|owner| {
                        promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
                    }) {
                        target_node_id = member_target;
                    } else if let Some(property_target) =
                        find_owner_at_or_after_token_with_node_type(
                            tree,
                            &owner_index,
                            token_index,
                            NodeType::Property,
                        )
                        .and_then(|owner| {
                            promote_owner_to_node_type_ancestor(
                                tree,
                                parents,
                                owner,
                                NodeType::Property,
                            )
                        })
                    {
                        target_node_id = property_target;
                    } else if let Some(declaration_target) =
                        find_owner_at_or_after_token_with_node_type(
                            tree,
                            &owner_index,
                            token_index,
                            NodeType::Declaration,
                        )
                        .and_then(|owner| {
                            promote_owner_to_declaration_ancestor(tree, parents, owner)
                        })
                    {
                        target_node_id = declaration_target;
                    }

                    break;
                }
            }

            let annotation = match ast_annotation {
                ast::Annotation::Doc { node, position } => Annotation::Doc {
                    node: *node,
                    position: doc_position_override.unwrap_or(*position),
                },
                ast::Annotation::Decorator { node, position } => {
                    let mut position = *position;

                    // decorators that start on the owner's line should stay inline
                    let owner_span = tree.get_span_by_id(target_node_id);
                    let decorator_starts_on_owner_line = annotation_span.end
                        > annotation_span.start
                        && file
                            .is_same_line(annotation_span.end.saturating_sub(1), owner_span.start);
                    let owner_is_declaration =
                        tree.get_node_type(target_node_id) == NodeType::Declaration;
                    let owner_is_default_export_value =
                        declaration_is_default_export_value(target_node_id);
                    if position == AnnotationPosition::BlockPrefix
                        && decorator_starts_on_owner_line
                        && owner_is_declaration
                        && owner_is_default_export_value
                    {
                        position = AnnotationPosition::LinePrefix;
                    }

                    Annotation::Decorator {
                        node: *node,
                        position,
                    }
                }
            };

            let local_id = LocalNodeId::new(entries.len() as u32);
            entries.push(FormatterAnnotationEntry {
                annotation,
                span: annotation_span,
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

/// Resolve one line comment after ternary `:` by normalizing the preceding owner seam.
pub(crate) fn attach_line_comment_after_ternary_colon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if let Some(target_owner) = preceding_owner {
        let target_owner =
            normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
        if tree.get_node_type(target_owner) == NodeType::Expression {
            let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
            return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
        }
    }

    let target_owner = following_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
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
    preceding_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let target_owner = token_before_span.and_then(|comma_token| {
        let search_start = comma_token.span.start.saturating_sub(1);
        (search_start < comma_token.span.start).then(|| {
            find_smallest_owner_enclosing_range(tree, search_start, comma_token.span.start)
        })?
    });
    let target_owner = target_owner.or(preceding_owner)?;

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

/// Resolve one star comment before `:` by attaching to tree-expression following or preceding boundary owner.
pub(crate) fn attach_star_comment_before_ternary_colon(
    tree: &NodeTree,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let preceding_target_owner =
        preceding_owner.filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)?;

    let following_prefers_tree_expression = following_owner.is_some_and(|owner| {
        tree.get_node_type(owner) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(owner)),
                Expression::TreeExpression { .. }
            )
    });
    if following_prefers_tree_expression && let Some(target_owner) = following_owner {
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    let target_expression = LocalNodeId::<Expression>::new(preceding_target_owner);
    if let Expression::Parenthesized { expression } = tree.get(target_expression) {
        let target_owner = normalize_formatter_trivia_target_owner(tree, expression.id);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, preceding_target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Attach inline multiline block comments that terminate one line.
fn try_attach_multiline_inline_block_comment_before_line_end(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.comment_is_multiline_star || !seam.has_trailing_newline || seam.has_leading_newline {
        return None;
    }

    let tree = context.tree;
    if seam.token_before_is(TokenType::OpenBrace)
        && !seam.token_after_is(TokenType::CloseBrace)
        && let Some(target_node) = fallback_following_owner(context, owners)
    {
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    let preceding_owner_from_token_before = context
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let preceding_owner = preceding_owner_from_token_before.or(owners.preceding);
    let parents: &NodeParentIndex = context.parents;
    let token_before_span = context.token_before_span.map(|token| token.span);

    if seam.token_after_is(TokenType::Semicolon)
        && let Some(target_node) = preceding_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    if let Some(preceding_owner) = preceding_owner {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, preceding_owner, token_before_span);
        return Some((Some(target_node), AnnotationPosition::BlockPostfix));
    }

    None
}

/// Attach comment-only file trivia to one enclosing owner.
fn try_attach_comment_only_file_fallback(
    context: &CommentSeamContext<'_>,
) -> Option<CommentAttachment> {
    if context.token_before.is_some() || context.token_after.is_some() {
        return None;
    }

    let tree = context.tree;
    let target_node = find_smallest_owner_enclosing_range(
        tree,
        context.trivia.span.start,
        context.trivia.span.end,
    )?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockPrefix))
}

/// Return one following owner normalized to the next token start.
fn fallback_following_owner(
    context: &CommentSeamContext<'_>,
    owners: CommentAttachmentNeighbors,
) -> Option<u32> {
    let tree = context.tree;
    let target_node = following_owner_with_token_after_fallback(tree, context, owners.following)?;
    let token_after_span = context.token_after_span.map(|token| token.span);
    let target_node = token_after_span
        .map(|span| promote_owner_by_shared_start(tree, context.parents, target_node, span.start))
        .unwrap_or(target_node);
    Some(normalize_formatter_trivia_target_owner(tree, target_node))
}

/// Return one preceding owner normalized to the previous token end.
fn fallback_preceding_owner(
    context: &CommentSeamContext<'_>,
    owners: CommentAttachmentNeighbors,
) -> Option<u32> {
    let tree = context.tree;
    let target_node = owners.preceding.or_else(|| {
        context
            .token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
    })?;
    let token_before_span = context.token_before_span.map(|token| token.span);
    Some(normalize_owner_with_shared_end(
        tree,
        context.parents,
        target_node,
        token_before_span,
    ))
}

/// Return whether one seam comment is an ignore directive line comment.
fn seam_comment_is_ignore_directive(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    seam.comment_is_line && comment_directive_is_ignore(context.trivia.directive)
}

/// Attach one own-line comment with one canonical placement fallback.
fn attach_default_own_line_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let tree = context.tree;
    let comment_is_ignore_directive = seam_comment_is_ignore_directive(context, seam);

    if comment_is_ignore_directive
        && let Some(target_node) = fallback_following_owner(context, owners)
    {
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    if let Some(target_node) = fallback_following_owner(context, owners) {
        let position = if seam.comment_is_line {
            AnnotationPosition::LinePrefix
        } else {
            AnnotationPosition::BlockPrefix
        };
        return Some((Some(target_node), position));
    }

    if let Some(target_node) = fallback_preceding_owner(context, owners) {
        let position = if seam.comment_is_line {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::BlockPostfix
        };
        return Some((Some(target_node), position));
    }

    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache)?;
    let enclosing_owner = normalize_formatter_trivia_target_owner(tree, enclosing_owner);
    Some((Some(enclosing_owner), AnnotationPosition::BlockInfix))
}

/// Attach one end-of-line comment with one canonical placement fallback.
fn attach_default_end_of_line_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let tree = context.tree;
    if let Some(target_node) = owners.preceding {
        let token_before_span = context.token_before_span.map(|token| token.span);
        let keep_literal_before_close_parenthesis = seam
            .token_after_is(TokenType::CloseParenthesis)
            && seam.token_before_is(TokenType::Literal)
            && !seam.token_before_is(TokenType::Comma);
        let should_keep_terminal_preceding_owner = keep_literal_before_close_parenthesis;
        let target_node = if should_keep_terminal_preceding_owner {
            target_node
        } else {
            normalize_owner_with_shared_end(tree, context.parents, target_node, token_before_span)
        };
        let position = if keep_literal_before_close_parenthesis {
            AnnotationPosition::LinePostfix
        } else {
            AnnotationPosition::LinePostfixBoundary
        };
        return Some((Some(target_node), position));
    }

    if let Some(target_node) = fallback_following_owner(context, owners) {
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache)?;
    let enclosing_owner = normalize_formatter_trivia_target_owner(tree, enclosing_owner);
    Some((Some(enclosing_owner), AnnotationPosition::BlockInfix))
}

/// Attach one remaining comment with one canonical placement fallback.
fn attach_default_remaining_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let tree = context.tree;

    // separators and operators that bind to the left keep trailing ownership
    if seam.token_after_prefers_preceding
        && let Some(target_node) = fallback_preceding_owner(context, owners)
    {
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // right-binding seams keep prefix ownership
    if seam.seam_binds_right
        && let Some(target_node) = fallback_following_owner(context, owners)
    {
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // default stable ownership: prefer preceding before following
    if let Some(target_node) = fallback_preceding_owner(context, owners) {
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }
    if let Some(target_node) = fallback_following_owner(context, owners) {
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache)?;
    let enclosing_owner = normalize_formatter_trivia_target_owner(tree, enclosing_owner);
    Some((Some(enclosing_owner), AnnotationPosition::BlockInfix))
}

/// Resolve the default comment trivia rules after specialized seam cases.
pub(crate) fn attach_comment_default(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> CommentAttachment {
    let placement = classify_comment_placement(context, seam);

    // own-line placement
    if placement == CommentPlacement::OwnLine {
        if let Some(attachment) =
            attach_own_line_comment(context, seam, enclosing_owner_cache, owners)
        {
            return attachment;
        }

        if let Some(attachment) =
            attach_default_own_line_comment(context, seam, enclosing_owner_cache, owners)
        {
            return attachment;
        }
    }

    // end-of-line placement
    if placement == CommentPlacement::EndOfLine {
        if let Some(attachment) =
            try_attach_multiline_inline_block_comment_before_line_end(context, seam, owners)
        {
            return attachment;
        }

        if let Some(attachment) =
            attach_end_of_line_comment(context, seam, enclosing_owner_cache, owners)
        {
            return attachment;
        }

        if let Some(attachment) =
            attach_default_end_of_line_comment(context, seam, enclosing_owner_cache, owners)
        {
            return attachment;
        }
    }

    // remaining placement
    if placement == CommentPlacement::Remaining {
        if let Some(attachment) = attach_remaining_comment(context, seam, owners) {
            return attachment;
        }

        if let Some(attachment) =
            attach_default_remaining_comment(context, seam, enclosing_owner_cache, owners)
        {
            return attachment;
        }
    }

    // comment only files can still anchor to one enclosing owner
    if let Some(attachment) = try_attach_comment_only_file_fallback(context) {
        return attachment;
    }

    (None, AnnotationPosition::BlockInfix)
}

/// Return one empty-statement body owner when one seam is before its semicolon.
fn empty_statement_body_owner_before_semicolon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<u32> {
    if !seam.token_after_is(ast::TokenType::Semicolon) {
        return None;
    }

    let following_owner = following_owner?;
    let block_owner = if tree.get_node_type(following_owner) == NodeType::Block {
        following_owner
    } else {
        promote_owner_to_node_type_ancestor(tree, parents, following_owner, NodeType::Block)?
    };

    let block_id = LocalNodeId::<Block>::new(block_owner);
    let block = tree.get(block_id);
    if block.format != BlockFormat::Implicit || !block.expressions.is_empty() {
        return None;
    }

    Some(block_owner)
}

/// Resolve one following owner with one token-after fallback owner.
pub(crate) fn following_owner_with_token_after_fallback(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    following_owner: Option<u32>,
) -> Option<u32> {
    following_owner.or_else(|| {
        context
            .token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
    })
}

/// Attach one seam comment that appears before one empty-statement body semicolon.
pub(crate) fn try_attach_comment_before_empty_statement_semicolon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let target_owner =
        empty_statement_body_owner_before_semicolon(tree, parents, seam, following_owner)?;
    let position = if seam.comment_is_line {
        AnnotationPosition::LinePrefix
    } else {
        AnnotationPosition::BlockPrefix
    };
    Some((Some(target_owner), position))
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
