use ast::{
    AnnotationPosition, Argument, CommentDirective, DependencyMode, Expression, Keyword,
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
    is_open_delimiter_token, previous_non_newline_token_index,
};
use super::declaration::try_attach_comment_declaration;
use super::endofline::attach_end_of_line_comment;
use super::expression::try_attach_comment_expression;
use super::facts::{
    next_non_trivia_token_index, previous_non_trivia_token_index, token_type_is_comment_trivia,
    token_type_is_trivia, token_type_is_whitespace_trivia,
};
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

/// Return whether one annotation span covers multiple source lines.
fn annotation_span_is_multiline(file: &File, annotation_span: Span) -> bool {
    annotation_span.start < annotation_span.end
        && !file.is_same_line(annotation_span.start, annotation_span.end.saturating_sub(1))
}

/// Resolve prefix position for one annotation anchored to one preceding token offset.
fn annotation_prefix_position_for_anchor(
    file: &File,
    anchor_offset: u32,
    annotation_span: Span,
) -> AnnotationPosition {
    let annotation_starts_on_anchor_line = file.is_same_line(anchor_offset, annotation_span.start);
    let annotation_is_multiline = annotation_span_is_multiline(file, annotation_span);
    if annotation_starts_on_anchor_line && !annotation_is_multiline {
        return AnnotationPosition::LinePrefix;
    }

    AnnotationPosition::BlockPrefix
}

/// Resolve assignment rhs owner from one assign token span.
fn assignment_rhs_owner_from_assign_token(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    assign_token_span: Span,
) -> Option<u32> {
    let owner_id = find_smallest_owner_enclosing_token(tree, assign_token_span)?;

    // expression assignments route to rhs expressions directly
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

    // declaration type assignments route to declaration values
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
}

/// Resolve fallback rhs expression owner after one annotation span.
fn assignment_fallback_expression_owner_after_annotation(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    token_after_annotation_index: usize,
    token_after_span: Span,
) -> Option<u32> {
    find_preferred_owner_starting_at(tree, token_after_span)
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
        })
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

    // assignment token span anchors rhs ownership and line-prefix decisions
    let previous_token_span = tokens[previous_index].span;

    // next semantic token after annotation drives rhs fallback ownership
    let (token_after_annotation_index, token_after_span, has_intervening_comment_trivia) =
        next_semantic_token_after_span(tokens, annotation_span)?;
    if has_intervening_comment_trivia || annotation_span.start >= token_after_span.start {
        return None;
    }

    // primary rhs owner comes from the assignment seam
    let assignment_owner =
        assignment_rhs_owner_from_assign_token(tree, parents, previous_token_span);

    // fallback rhs owner starts at the first expression after annotation
    let fallback_expression_owner = assignment_fallback_expression_owner_after_annotation(
        tree,
        parents,
        owner_index,
        token_after_annotation_index,
        token_after_span,
    );

    let expression_owner = assignment_owner.or(fallback_expression_owner)?;
    let expression_owner =
        promote_rhs_expression_owner(tree, parents, expression_owner, Some(token_after_span));

    // inline assignment seam doc comments stay line-prefix
    let position =
        annotation_prefix_position_for_anchor(file, previous_token_span.start, annotation_span);

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

/// Return rhs owner for one doc annotation that follows one opening parenthesis seam.
fn parenthesized_rhs_owner_for_doc_annotation(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(u32, AnnotationPosition)> {
    let annotation_token_index =
        tokens.partition_point(|token| token.span.start < annotation_span.start);
    if annotation_token_index >= tokens.len() {
        return None;
    }

    let previous_index = previous_non_trivia_token_index(tokens, annotation_token_index)?;
    if tokens[previous_index].token.ty != TokenType::OpenParenthesis {
        return None;
    }
    let previous_token_span = tokens[previous_index].span;

    let (token_after_annotation_index, token_after_span, has_intervening_comment_trivia) =
        next_semantic_token_after_span(tokens, annotation_span)?;
    if has_intervening_comment_trivia || annotation_span.start >= token_after_span.start {
        return None;
    }

    let expression_owner = find_smallest_owner_enclosing_token(tree, token_after_span)
        .or_else(|| find_owner_at_or_after_token(tree, tokens, token_after_annotation_index))
        .filter(|owner_id| tree.get_node_type(*owner_id) == NodeType::Expression)?;
    let position =
        annotation_prefix_position_for_anchor(file, previous_token_span.start, annotation_span);

    Some((expression_owner, position))
}

/// Return the next semantic token after one annotation span.
///
/// The boolean indicates whether one comment trivia token exists before that semantic token.
fn next_semantic_token_after_span(
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(usize, Span, bool)> {
    let mut token_index = tokens.partition_point(|token| token.span.start < annotation_span.end);
    let mut has_intervening_comment_trivia = false;

    while token_index < tokens.len() {
        let token = tokens[token_index];
        let token_type = token.token.ty;

        if token_type_is_whitespace_trivia(token_type) {
            token_index += 1;
            continue;
        }

        if token_type_is_comment_trivia(token_type) {
            has_intervening_comment_trivia = true;
            token_index += 1;
            continue;
        }

        return Some((token_index, token.span, has_intervening_comment_trivia));
    }

    None
}

/// Return whether all semantic tokens between one source range are whitespace trivia.
fn token_range_is_whitespace_trivia_only(tokens: &[TokenSpan], start: u32, end: u32) -> bool {
    let token_start_index = tokens.partition_point(|token| token.span.start < start);
    for token in tokens.iter().skip(token_start_index) {
        if token.span.start >= end {
            break;
        }

        if token.span.end <= start {
            continue;
        }

        if !token_type_is_whitespace_trivia(token.token.ty) {
            return false;
        }
    }

    true
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

/// Return start and end token lookup maps by source offset.
fn formatter_token_offset_maps(
    semantic_tokens: &[TokenSpan],
) -> (FxHashMap<u32, usize>, FxHashMap<u32, usize>) {
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

    (start_token_by_offset, end_token_by_offset)
}

/// Return owner rank used to break ties between equally-sized owners.
fn formatter_owner_kind_rank(tree: &NodeTree, node_id: u32) -> u8 {
    if tree.get_node_type(node_id) == NodeType::Expression {
        return 1;
    }

    0
}

/// Update start and end owner slots for one node span.
fn update_formatter_owner_slots_for_node(
    tree: &NodeTree,
    start_token_by_offset: &FxHashMap<u32, usize>,
    end_token_by_offset: &FxHashMap<u32, usize>,
    owner_start_by_token: &mut [Option<u32>],
    owner_end_by_token: &mut [Option<u32>],
    owner_start_length_by_token: &mut [u32],
    owner_end_length_by_token: &mut [u32],
    owner_start_kind_rank_by_token: &mut [u8],
    owner_end_kind_rank_by_token: &mut [u8],
    node_id: u32,
) {
    let span = tree.get_span_by_id(node_id);
    let length = span.end.saturating_sub(span.start);
    let node_kind_rank = formatter_owner_kind_rank(tree, node_id);

    if let Some(token_index) = start_token_by_offset.get(&span.start).copied() {
        update_best_formatter_owner_slot(
            owner_start_by_token,
            owner_start_length_by_token,
            owner_start_kind_rank_by_token,
            token_index,
            node_id,
            length,
            node_kind_rank,
        );
    }

    if let Some(token_index) = end_token_by_offset.get(&span.end).copied() {
        update_best_formatter_owner_slot(
            owner_end_by_token,
            owner_end_length_by_token,
            owner_end_kind_rank_by_token,
            token_index,
            node_id,
            length,
            node_kind_rank,
        );
    }
}

/// Build best start and end owner ids for attachable semantic token indexes.
fn formatter_owner_start_end_by_token(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
) -> (Vec<Option<u32>>, Vec<Option<u32>>) {
    let (start_token_by_offset, end_token_by_offset) = formatter_token_offset_maps(semantic_tokens);

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

        update_formatter_owner_slots_for_node(
            tree,
            &start_token_by_offset,
            &end_token_by_offset,
            &mut owner_start_by_token,
            &mut owner_end_by_token,
            &mut owner_start_length_by_token,
            &mut owner_end_length_by_token,
            &mut owner_start_kind_rank_by_token,
            &mut owner_end_kind_rank_by_token,
            node_id,
        );

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

/// Normalize one boundary-before token index to the nearest non-trivia token at or before it.
fn normalize_boundary_before_token_index(
    semantic_tokens: &[TokenSpan],
    token_index: Option<usize>,
) -> Option<usize> {
    let token_index = token_index?;
    let token_type = semantic_tokens.get(token_index)?.token.ty;
    if !token_type_is_trivia(token_type) {
        return Some(token_index);
    }

    previous_non_trivia_token_index(semantic_tokens, token_index)
}

/// Normalize one boundary-after token index to the nearest non-trivia token at or after it.
fn normalize_boundary_after_token_index(
    semantic_tokens: &[TokenSpan],
    token_index: Option<usize>,
) -> Option<usize> {
    let token_index = token_index?;
    let token_type = semantic_tokens.get(token_index)?.token.ty;
    if !token_type_is_trivia(token_type) {
        return Some(token_index);
    }

    next_non_trivia_token_index(semantic_tokens, token_index)
}

/// Resolve the nearest non-trivia token index before one source offset.
fn previous_non_trivia_token_index_before_offset(
    semantic_tokens: &[TokenSpan],
    offset: u32,
) -> Option<usize> {
    let token_index = semantic_tokens.partition_point(|token| token.span.start < offset);
    previous_non_trivia_token_index(semantic_tokens, token_index)
}

/// Resolve the nearest non-trivia token index at or after one source offset.
fn next_non_trivia_token_index_at_or_after_offset(
    semantic_tokens: &[TokenSpan],
    offset: u32,
) -> Option<usize> {
    let mut token_index = semantic_tokens.partition_point(|token| token.span.start < offset);
    while token_index < semantic_tokens.len() {
        let token_type = semantic_tokens[token_index].token.ty;
        if !token_type_is_trivia(token_type) {
            return Some(token_index);
        }

        token_index += 1;
    }

    None
}

/// Return whether one expression is delimited by one matching token pair.
fn expression_matches_delimiter_pair(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    open_delimiter: TokenType,
    close_delimiter: TokenType,
) -> bool {
    matches!(
        (open_delimiter, close_delimiter, tree.get(expression_id)),
        (
            TokenType::OpenBracket,
            TokenType::CloseBracket,
            Expression::ArrayExpression { .. }
        ) | (
            TokenType::OpenBrace,
            TokenType::CloseBrace,
            Expression::ObjectExpression { .. }
        )
    )
}

/// Normalize one delimiter-interior container owner to the literal expression when wrapped by an argument.
fn delimiter_interior_container_owner(
    tree: &NodeTree,
    container_owner: u32,
    open_delimiter: TokenType,
    close_delimiter: TokenType,
) -> u32 {
    if tree.get_node_type(container_owner) != NodeType::Argument {
        return container_owner;
    }

    let argument_id = LocalNodeId::<Argument>::new(container_owner);
    let value_expression_id = argument_value_expression_id(tree, argument_id);
    if expression_matches_delimiter_pair(tree, value_expression_id, open_delimiter, close_delimiter)
    {
        return value_expression_id.id;
    }

    container_owner
}

/// Return whether one owner is one empty import or export dependency expression.
pub(crate) fn is_empty_dependency_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    match tree.get(expression_id) {
        Expression::Import { source, items, .. } => {
            *source != ast::ImportSource::ImportCall && items.is_empty()
        }
        Expression::Export { items, .. } => items.is_empty(),
        _ => false,
    }
}

/// Return seam boundary token spans when both sides exist.
fn delimiter_seam_boundary_tokens(ctx: &CommentSeamContext<'_>) -> Option<(TokenSpan, TokenSpan)> {
    let token_before_span = ctx.token_before_span?;
    let token_after_span = ctx.token_after_span?;
    Some((token_before_span, token_after_span))
}

/// Return whether boundary tokens form one matching delimiter pair.
fn delimiter_seam_is_matching_pair(
    token_before_span: TokenSpan,
    token_after_span: TokenSpan,
) -> bool {
    is_open_delimiter_token(token_before_span.token.ty)
        && is_close_delimiter_token(token_after_span.token.ty)
        && delimiters_match(token_before_span.token.ty, token_after_span.token.ty)
}

/// Resolve one delimiter interior container owner from boundary tokens.
fn delimiter_interior_container_candidate(
    tree: &NodeTree,
    token_before_span: TokenSpan,
    token_after_span: TokenSpan,
) -> Option<u32> {
    find_preferred_owner_starting_at(tree, token_before_span.span)
        .filter(|owner_id| tree.get_span_by_id(*owner_id).end >= token_after_span.span.end)
        .or_else(|| {
            find_smallest_owner_enclosing_range(
                tree,
                token_before_span.span.start,
                token_after_span.span.end,
            )
        })
}

/// Resolve final target node for delimiter interior attachment.
fn delimiter_interior_target_node(tree: &NodeTree, container_owner: u32) -> u32 {
    if tree.get_node_type(container_owner) == NodeType::Expression {
        let expression_id = LocalNodeId::<ast::Expression>::new(container_owner);
        if let ast::Expression::Block(block_id) = tree.get(expression_id) {
            return block_id.id;
        }
    }

    normalize_formatter_trivia_target_owner(tree, container_owner)
}

/// Try to attach one comment inside matching delimiters as container infix trivia.
fn try_attach_comment_delimiter_interior(
    tree: &NodeTree,
    ctx: &CommentSeamContext<'_>,
) -> Option<CommentAttachment> {
    let (token_before_span, token_after_span) = delimiter_seam_boundary_tokens(ctx)?;

    // only matching delimiter seams can host delimiter-interior comments
    if !delimiter_seam_is_matching_pair(token_before_span, token_after_span) {
        return None;
    }

    // resolve the best container owner spanning the seam
    let container_owner =
        delimiter_interior_container_candidate(tree, token_before_span, token_after_span)?;
    let container_owner = delimiter_interior_container_owner(
        tree,
        container_owner,
        token_before_span.token.ty,
        token_after_span.token.ty,
    );

    // empty import and export objects use dedicated handling outside interior capture
    if token_before_span.token.ty == TokenType::OpenBrace
        && token_after_span.token.ty == TokenType::CloseBrace
        && is_empty_dependency_expression_owner(tree, container_owner)
    {
        return None;
    }

    let target_node = delimiter_interior_target_node(tree, container_owner);

    Some((Some(target_node), ast::AnnotationPosition::BlockInfix))
}

/// Try to attach one seam comment between parameter name and type to the parameter owner.
fn try_attach_comment_parameter_type_boundary(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ctx: &CommentSeamContext<'_>,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !ctx
        .token_after_span
        .is_some_and(|token| token.token.ty == TokenType::Colon)
    {
        return None;
    }

    let owner_from_seam_tokens =
        ctx.token_before_span
            .zip(ctx.token_after_span)
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
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    attachment: CommentAttachment,
) -> CommentAttachment {
    let (owner, position) = attachment;
    let Some(owner_id) = owner else {
        return (owner, position);
    };

    // own-line line comments inside empty object literal arguments stay on the object expression
    if let Some(attachment) =
        normalize_empty_object_argument_own_line_comment_attachment(tree, seam, owner_id)
    {
        return attachment;
    }

    // inline block comments before object-literal argument values stay on the object expression
    if let Some(attachment) =
        normalize_inline_object_argument_prefix_comment_attachment(tree, seam, owner_id)
    {
        return attachment;
    }

    // own-line line comments between a closing delimiter and comma belong to the preceding value boundary
    if let Some(attachment) =
        normalize_own_line_closing_delimiter_comma_attachment(tree, seam, owner_id)
    {
        return attachment;
    }

    normalize_inline_trailing_comma_before_close_brace_attachment(
        tree, parents, ctx, seam, owner_id, owner, position,
    )
}

/// Return one argument value expression id.
#[inline]
fn argument_value_expression_id(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> LocalNodeId<Expression> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => {
            unreachable!(
                "formatter does not yet derive one value expression for argument error slots"
            )
        }
    }
}

/// Attach own-line line comments inside empty object literal arguments to the object expression.
fn normalize_empty_object_argument_own_line_comment_attachment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    owner_id: u32,
) -> Option<CommentAttachment> {
    if !seam.comment_is_line
        || !seam.has_leading_newline
        || !seam.token_before_is(TokenType::OpenBrace)
        || !seam.token_after_is(TokenType::CloseBrace)
        || tree.get_node_type(owner_id) != NodeType::Argument
    {
        return None;
    }

    let argument_id = LocalNodeId::<Argument>::new(owner_id);
    let value_id = argument_value_expression_id(tree, argument_id);
    if !matches!(
        tree.get(value_id),
        Expression::ObjectExpression { properties, .. } if properties.is_empty()
    ) {
        return None;
    }

    let value_owner = normalize_formatter_trivia_target_owner(tree, value_id.id);
    Some((Some(value_owner), AnnotationPosition::BlockInfix))
}

/// Attach inline block comments before object-literal argument values to the object expression.
fn normalize_inline_object_argument_prefix_comment_attachment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    owner_id: u32,
) -> Option<CommentAttachment> {
    if !seam.comment_is_star
        || seam.has_leading_newline
        || seam.has_trailing_newline
        || !seam.token_after_is(TokenType::OpenBrace)
        || tree.get_node_type(owner_id) != NodeType::Argument
    {
        return None;
    }

    let argument_id = LocalNodeId::<Argument>::new(owner_id);
    let value_id = argument_value_expression_id(tree, argument_id);
    if !matches!(tree.get(value_id), Expression::ObjectExpression { .. }) {
        return None;
    }

    let value_owner = normalize_formatter_trivia_target_owner(tree, value_id.id);
    Some((Some(value_owner), AnnotationPosition::LinePrefix))
}

/// Attach own-line line comments between closing delimiter and comma to the preceding boundary owner.
fn normalize_own_line_closing_delimiter_comma_attachment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    owner_id: u32,
) -> Option<CommentAttachment> {
    if !seam.comment_is_line
        || !seam.has_leading_newline
        || !seam.token_after_is(TokenType::Comma)
        || !(seam.token_before_is(TokenType::CloseBrace)
            || seam.token_before_is(TokenType::CloseBracket)
            || seam.token_before_is(TokenType::CloseParenthesis))
    {
        return None;
    }

    let boundary_owner = normalize_formatter_trivia_target_owner(tree, owner_id);
    Some((
        Some(boundary_owner),
        AnnotationPosition::LinePostfixBoundary,
    ))
}

/// Attach inline trailing comma comments before close brace to the trailing property boundary.
fn normalize_inline_trailing_comma_before_close_brace_attachment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owner_id: u32,
    owner: Option<u32>,
    position: AnnotationPosition,
) -> CommentAttachment {
    if !seam.comment_is_line
        || seam.has_leading_newline
        || !seam.token_before_is(TokenType::Comma)
        || !seam.token_after_is(TokenType::CloseBrace)
        || tree.get_node_type(owner_id) != NodeType::Argument
    {
        return (owner, position);
    }

    let Some(member_owner) = trailing_property_owner_before_comma(tree, parents, ctx) else {
        return (owner, position);
    };

    let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
    (
        Some(member_owner),
        ast::AnnotationPosition::LinePostfixBoundary,
    )
}

/// Return one property owner directly before one comma token at seam left.
fn trailing_property_owner_before_comma(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ctx: &CommentSeamContext<'_>,
) -> Option<u32> {
    let comma_token = ctx.token_before_span?;
    let search_start = comma_token.span.start.saturating_sub(1);
    if search_start >= comma_token.span.start {
        return None;
    }

    let candidate_owner =
        find_smallest_owner_enclosing_range(tree, search_start, comma_token.span.start)?;
    promote_owner_to_node_type_ancestor(tree, parents, candidate_owner, NodeType::Property)
}

/// Mutable dispatch ctx for one comment seam attachment pipeline.
struct CommentAttachmentDispatchContext<'a, 'cache> {
    /// The syntax tree.
    tree: &'a NodeTree,
    /// The precomputed owner index.
    owner_index: &'a FormatterTriviaOwnerIndex,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// The seam ctx.
    seam_ctx: &'a CommentSeamContext<'a>,
    /// The derived seam facts.
    seam: &'a CommentSeamData,
    /// Neighbor owner candidates.
    owners: CommentAttachmentNeighbors,
    /// Mutable enclosing owner cache.
    enclosing_owner_cache: &'cache mut CommentEnclosingOwnerCache,
}

/// One attachment handler in the seam dispatch pipeline.
type CommentAttachmentHandler =
    fn(&mut CommentAttachmentDispatchContext<'_, '_>) -> Option<CommentAttachment>;

/// Ordered seam attachment handlers for comment trivia routing.
const COMMENT_ATTACHMENT_HANDLERS: &[CommentAttachmentHandler] = &[
    attach_comment_delimiter_interior_dispatch,
    attach_comment_parameter_type_boundary_dispatch,
    attach_comment_expression_dispatch,
    attach_comment_statement_prefix_dispatch,
    attach_comment_declaration_dispatch,
    attach_comment_statement_suffix_dispatch,
    attach_comment_assignment_dispatch,
    attach_comment_block_body_dispatch,
    attach_comment_default_dispatch,
];

/// Run one ordered list of seam attachment handlers.
fn run_comment_attachment_handlers(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
    handlers: &[CommentAttachmentHandler],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Attach one delimiter interior seam comment.
fn attach_comment_delimiter_interior_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_delimiter_interior(ctx.tree, ctx.seam_ctx)
}

/// Attach one parameter type boundary seam comment.
fn attach_comment_parameter_type_boundary_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_parameter_type_boundary(ctx.tree, ctx.parents, ctx.seam_ctx, ctx.owners)
}

/// Attach one expression seam comment.
fn attach_comment_expression_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_expression(
        ctx.tree,
        ctx.owner_index,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.owners,
    )
}

/// Attach one statement prefix seam comment.
fn attach_comment_statement_prefix_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_statement_prefix(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.owners,
    )
}

/// Attach one declaration seam comment.
fn attach_comment_declaration_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_declaration(
        ctx.tree,
        ctx.owner_index,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners,
    )
}

/// Attach one statement suffix seam comment.
fn attach_comment_statement_suffix_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_statement_suffix(ctx.tree, ctx.parents, ctx.seam_ctx, ctx.seam, ctx.owners)
}

/// Attach one assignment seam comment.
fn attach_comment_assignment_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_assignment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.owners,
    )
}

/// Attach one block body seam comment.
fn attach_comment_block_body_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_block_body(ctx.tree, ctx.seam, ctx.owners)
}

/// Attach one default seam comment fallback.
fn attach_comment_default_dispatch(
    ctx: &mut CommentAttachmentDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    Some(attach_comment_default(
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.owners,
    ))
}

/// Boundary token facts derived from one comment seam.
struct CommentSeamBoundaryTokens {
    /// The normalized non-trivia token index before the seam.
    token_before_index: Option<usize>,
    /// The normalized non-trivia token index after the seam.
    token_after_index: Option<usize>,
    /// The token span before the seam.
    token_before_span: Option<TokenSpan>,
    /// The token span after the seam.
    token_after_span: Option<TokenSpan>,
}

/// Return following owner from exact and nearest start-owner indexes.
fn seam_following_owner_from_indexes(
    owner_index: &FormatterTriviaOwnerIndex,
    token_after_index: Option<usize>,
) -> Option<u32> {
    let following_owner = token_after_index.and_then(|index| {
        owner_index
            .owner_start_by_token
            .get(index)
            .and_then(|owner| *owner)
    });
    if following_owner.is_some() {
        return following_owner;
    }

    token_after_index.and_then(|index| {
        owner_index
            .nearest_owner_start_by_token
            .get(index)
            .and_then(|owner| *owner)
    })
}

/// Return preceding owner from exact and nearest end-owner indexes.
fn seam_preceding_owner_from_indexes(
    owner_index: &FormatterTriviaOwnerIndex,
    token_before_index: Option<usize>,
) -> Option<u32> {
    let preceding_owner = token_before_index.and_then(|index| {
        owner_index
            .owner_end_by_token
            .get(index)
            .and_then(|owner| *owner)
    });
    if preceding_owner.is_some() {
        return preceding_owner;
    }

    token_before_index.and_then(|index| {
        owner_index
            .nearest_owner_end_by_token
            .get(index)
            .and_then(|owner| *owner)
    })
}

/// Return following owner fallback from token-after span.
fn seam_following_owner_from_span(tree: &NodeTree, token_after_span: TokenSpan) -> Option<u32> {
    find_preferred_owner_starting_at(tree, token_after_span.span)
        .or_else(|| find_smallest_owner_enclosing_token(tree, token_after_span.span))
}

/// Return preceding owner fallback from token-before span.
fn seam_preceding_owner_from_span(tree: &NodeTree, token_before_span: TokenSpan) -> Option<u32> {
    find_smallest_owner_enclosing_token(tree, token_before_span.span)
}

/// Resolve normalized seam boundary token indexes and spans for one comment trivia.
fn resolve_seam_boundary_tokens(
    semantic_tokens: &[TokenSpan],
    trivia: ast::CommentTrivia,
) -> CommentSeamBoundaryTokens {
    let token_before = normalize_boundary_before_token_index(
        semantic_tokens,
        decode_token_index(trivia.boundary.token_before),
    )
    .or_else(|| previous_non_trivia_token_index_before_offset(semantic_tokens, trivia.span.start));
    let token_after = normalize_boundary_after_token_index(
        semantic_tokens,
        decode_token_index(trivia.boundary.token_after),
    )
    .or_else(|| next_non_trivia_token_index_at_or_after_offset(semantic_tokens, trivia.span.end));
    let token_before_span = token_before
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let token_after_span = token_after
        .and_then(|index| semantic_tokens.get(index))
        .copied();

    CommentSeamBoundaryTokens {
        token_before_index: token_before,
        token_after_index: token_after,
        token_before_span,
        token_after_span,
    }
}

/// Resolve preceding and following seam owners from boundary token facts.
fn resolve_seam_boundary_owners(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    seam_tokens: &CommentSeamBoundaryTokens,
) -> CommentAttachmentNeighbors {
    let mut following_owner =
        seam_following_owner_from_indexes(owner_index, seam_tokens.token_after_index);

    let mut preceding_owner =
        seam_preceding_owner_from_indexes(owner_index, seam_tokens.token_before_index);

    // token spans close seam ownership gaps when index tables do not resolve
    if following_owner.is_none()
        && let Some(token_after_span) = seam_tokens.token_after_span
    {
        following_owner = seam_following_owner_from_span(tree, token_after_span);
    }

    // token spans close seam ownership gaps when index tables do not resolve
    if preceding_owner.is_none()
        && let Some(token_before_span) = seam_tokens.token_before_span
    {
        preceding_owner = seam_preceding_owner_from_span(tree, token_before_span);
    }

    CommentAttachmentNeighbors::new(preceding_owner, following_owner)
}

/// Build one comment seam ctx from boundary token facts.
fn build_comment_seam_context<'a>(
    file: &'a File,
    tree: &'a NodeTree,
    semantic_tokens: &'a [TokenSpan],
    token_keyword_by_span: &'a FxHashMap<Span, Option<Keyword>>,
    trivia: ast::CommentTrivia,
    parents: &'a NodeParentIndex,
    seam_tokens: &CommentSeamBoundaryTokens,
) -> CommentSeamContext<'a> {
    CommentSeamContext {
        file,
        tree,
        semantic_tokens,
        token_keyword_by_span,
        trivia,
        parents,
        token_before: seam_tokens.token_before_index,
        token_after: seam_tokens.token_after_index,
        token_before_span: seam_tokens.token_before_span,
        token_after_span: seam_tokens.token_after_span,
    }
}

/// Attach one comment seam using the ordered attachment pipeline.
fn attach_comment_for_seam(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    seam_ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> CommentAttachment {
    let mut enclosing_owner_cache = CommentEnclosingOwnerCache::default();
    let mut ctx = CommentAttachmentDispatchContext {
        tree,
        owner_index,
        parents,
        seam_ctx,
        seam,
        owners,
        enclosing_owner_cache: &mut enclosing_owner_cache,
    };
    let attachment = run_comment_attachment_handlers(&mut ctx, COMMENT_ATTACHMENT_HANDLERS);

    attachment.expect("comment attachment pipeline should always produce one attachment")
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
    let seam_tokens = resolve_seam_boundary_tokens(semantic_tokens, trivia);
    let owners = resolve_seam_boundary_owners(tree, owner_index, &seam_tokens);
    let ctx = build_comment_seam_context(
        file,
        tree,
        semantic_tokens,
        token_keyword_by_span,
        trivia,
        parents,
        &seam_tokens,
    );
    let seam = CommentSeamData::build(&ctx);
    let attachment = attach_comment_for_seam(tree, owner_index, parents, &ctx, &seam, owners);

    normalize_trailing_object_member_comment_attachment(tree, parents, &ctx, &seam, attachment)
}

/// Return whether one declaration owner is the default export value of one export expression chain.
fn declaration_expression_owner_for_declaration_target(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    target_node_id: u32,
) -> Option<u32> {
    if tree.get_node_type(target_node_id) != NodeType::Declaration {
        return None;
    }

    let declaration_expression_node_id = parents.get_by_id(target_node_id)?;
    if tree.get_node_type(declaration_expression_node_id) != NodeType::Expression {
        return None;
    }

    let declaration_expression_id = LocalNodeId::<Expression>::new(declaration_expression_node_id);
    let Expression::Declaration(declaration_id) = tree.get(declaration_expression_id) else {
        return None;
    };
    if declaration_id.id != target_node_id {
        return None;
    }

    Some(declaration_expression_node_id)
}

/// Return whether one export expression references one expression as its default item value.
fn export_expression_has_default_item_value(
    tree: &NodeTree,
    export_expression_node_id: u32,
    declaration_expression_node_id: u32,
) -> bool {
    if tree.get_node_type(export_expression_node_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(export_expression_node_id);
    let Expression::Export { items, .. } = tree.get(expression_id) else {
        return false;
    };

    items.iter().any(|item_id| {
        let item = tree.get(*item_id);
        item.mode == DependencyMode::Default
            && item
                .value
                .is_some_and(|value_id| value_id.id == declaration_expression_node_id)
    })
}

/// Return whether one declaration owner is the default export value of one export expression chain.
fn declaration_is_default_export_value(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    target_node_id: u32,
) -> bool {
    let Some(declaration_expression_node_id) =
        declaration_expression_owner_for_declaration_target(tree, parents, target_node_id)
    else {
        return false;
    };

    let mut ancestor_id = parents.get_by_id(declaration_expression_node_id);
    while let Some(node_id) = ancestor_id {
        if export_expression_has_default_item_value(tree, node_id, declaration_expression_node_id) {
            return true;
        }

        ancestor_id = parents.get_by_id(node_id);
    }

    false
}

/// Sort node local annotation ids by source position for deterministic rendering.
fn sort_node_annotation_ids_by_source(
    entries: &[FormatterAnnotationEntry],
    by_node_id: &mut [SmallVec<[LocalNodeId<Annotation>; 4]>],
) {
    for annotation_ids in by_node_id {
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
}

/// One resolved semantic-annotation target for formatter projection.
#[derive(Debug, Clone, Copy)]
struct SemanticAnnotationTarget {
    /// The resolved target node id.
    target_node_id: u32,
    /// Optional doc-position override from rhs attachment rules.
    doc_position_override: Option<AnnotationPosition>,
}

/// Return the first non-whitespace token index after one span.
fn first_non_whitespace_token_index_after_span(tokens: &[TokenSpan], span: Span) -> Option<usize> {
    let mut token_index = tokens.partition_point(|token| token.span.start < span.end);
    while let Some(token) = tokens.get(token_index) {
        if !matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
            return Some(token_index);
        }

        token_index += 1;
    }

    None
}

/// Resolve the declaration target for one doc comment directly before one decorator.
fn doc_decorator_declaration_target(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: usize,
) -> Option<u32> {
    let declaration_id = find_owner_at_or_after_token_with_node_type(
        tree,
        owner_index,
        token_index,
        NodeType::Declaration,
    )?;
    Some(declaration_id)
}

/// Resolve rhs projection target for one doc annotation in canonical priority order.
fn resolve_doc_rhs_projection_target(
    file: &File,
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(u32, AnnotationPosition)> {
    if let Some((trailing_owner, trailing_position)) = trailing_statement_owner_for_doc_annotation(
        file,
        tree,
        parents,
        owner_index,
        tokens,
        annotation_span,
    ) {
        return Some((trailing_owner, trailing_position));
    }

    if let Some((parenthesized_owner, parenthesized_position)) =
        parenthesized_rhs_owner_for_doc_annotation(file, tree, tokens, annotation_span)
    {
        return Some((parenthesized_owner, parenthesized_position));
    }

    assignment_like_rhs_owner_for_doc_annotation(
        file,
        tree,
        parents,
        owner_index,
        tokens,
        annotation_span,
    )
}

/// Resolve one semantic doc annotation target and optional position override.
fn resolve_doc_annotation_target(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    annotation_span: Span,
    default_target_node_id: u32,
) -> SemanticAnnotationTarget {
    let mut target = SemanticAnnotationTarget {
        target_node_id: default_target_node_id,
        doc_position_override: None,
    };

    // doc comments directly before decorators belong to the decorated declaration
    if let Some(token_index) = first_non_whitespace_token_index_after_span(tokens, annotation_span)
    {
        let token = tokens[token_index];
        if token.token.ty == TokenType::At
            && let Some(declaration_id) =
                doc_decorator_declaration_target(tree, owner_index, token_index)
        {
            target.target_node_id = declaration_id;
        }
    }

    // rhs projection order: trailing statement, parenthesized rhs, assignment rhs
    if let Some((target_node_id, position)) =
        resolve_doc_rhs_projection_target(file, tree, parents, owner_index, tokens, annotation_span)
    {
        target.target_node_id = target_node_id;
        target.doc_position_override = Some(position);
    }

    target
}

/// Resolve one decorator target owner from one first token after the annotation span.
fn resolve_decorator_target_at_token(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: usize,
) -> Option<u32> {
    if let Some(member_target) = find_owner_at_or_after_token_with_node_type(
        tree,
        owner_index,
        token_index,
        NodeType::Member,
    )
    .and_then(|owner| promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member))
    {
        return Some(member_target);
    }

    if let Some(property_target) = find_owner_at_or_after_token_with_node_type(
        tree,
        owner_index,
        token_index,
        NodeType::Property,
    )
    .and_then(|owner| promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Property))
    {
        return Some(property_target);
    }

    find_owner_at_or_after_token_with_node_type(
        tree,
        owner_index,
        token_index,
        NodeType::Declaration,
    )
    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
}

/// Resolve one semantic decorator annotation target.
fn resolve_decorator_annotation_target(
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    annotation_span: Span,
    default_target_node_id: u32,
) -> u32 {
    if tree.get_node_type(default_target_node_id) != NodeType::Expression {
        return default_target_node_id;
    }

    let Some(token_index) = first_non_whitespace_token_index_after_span(tokens, annotation_span)
    else {
        return default_target_node_id;
    };

    if let Some(target_node_id) =
        resolve_decorator_target_at_token(tree, parents, owner_index, token_index)
    {
        return target_node_id;
    }

    default_target_node_id
}

/// Build one formatter annotation from one parser semantic annotation.
fn semantic_formatter_annotation(
    file: &File,
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ast_annotation: &ast::Annotation,
    annotation_span: Span,
    target: SemanticAnnotationTarget,
) -> Annotation {
    match ast_annotation {
        ast::Annotation::Doc { node, position } => Annotation::Doc {
            node: *node,
            position: target.doc_position_override.unwrap_or(*position),
        },
        ast::Annotation::Decorator { node, position } => {
            let mut position = *position;

            // decorators that start on the owner's line should stay inline
            let owner_span = tree.get_span_by_id(target.target_node_id);
            let decorator_starts_on_owner_line = annotation_span.end > annotation_span.start
                && file.is_same_line(annotation_span.end.saturating_sub(1), owner_span.start);
            let owner_is_declaration =
                tree.get_node_type(target.target_node_id) == NodeType::Declaration;
            let owner_is_default_export_value =
                declaration_is_default_export_value(tree, parents, target.target_node_id);
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
    }
}

/// Projection storage for formatter annotations keyed by target node.
struct AnnotationProjectionStorage {
    /// All projected annotations in insertion order.
    entries: Vec<FormatterAnnotationEntry>,
    /// Per-node annotation ids in source-stable order.
    by_node_id: Vec<SmallVec<[LocalNodeId<Annotation>; 4]>>,
}

impl AnnotationProjectionStorage {
    /// Build empty projection storage for one tree node count.
    fn new(node_count: usize) -> Self {
        Self {
            entries: Vec::new(),
            by_node_id: vec![SmallVec::new(); node_count],
        }
    }

    /// Return whether one target node id exists in projection storage.
    fn has_target_node(&self, target_node_id: u32) -> bool {
        (target_node_id as usize) < self.by_node_id.len()
    }

    /// Push one projected annotation when the target node id exists.
    fn push_entry(&mut self, target_node_id: u32, annotation: Annotation, span: Span) -> bool {
        if !self.has_target_node(target_node_id) {
            return false;
        }

        let local_id = LocalNodeId::new(self.entries.len() as u32);
        self.entries
            .push(FormatterAnnotationEntry { annotation, span });
        self.by_node_id[target_node_id as usize].push(local_id);
        true
    }

    /// Sort node-local annotation ids by source span order.
    fn sort_by_source(&mut self) {
        sort_node_annotation_ids_by_source(&self.entries, &mut self.by_node_id);
    }

    /// Return owned entries and per-node ids.
    fn into_parts(
        self,
    ) -> (
        Vec<FormatterAnnotationEntry>,
        Vec<SmallVec<[LocalNodeId<Annotation>; 4]>>,
    ) {
        (self.entries, self.by_node_id)
    }
}

/// Push one comment trivia projection entry.
fn push_comment_trivia_projection_entry(
    storage: &mut AnnotationProjectionStorage,
    target_node_id: u32,
    trivia: ast::CommentTrivia,
    position: AnnotationPosition,
) -> bool {
    storage.push_entry(
        target_node_id,
        Annotation::Comment {
            node: trivia.comment,
            position,
        },
        trivia.span,
    )
}

/// Push one blank trivia projection entry.
fn push_blank_trivia_projection_entry(
    storage: &mut AnnotationProjectionStorage,
    target_node_id: u32,
    trivia: ast::BlankTrivia,
    position: AnnotationPosition,
) -> bool {
    storage.push_entry(
        target_node_id,
        Annotation::Blank {
            node: trivia.blank,
            position,
        },
        trivia.span,
    )
}

/// Project parser side semantic annotations into formatter annotation entries.
fn semantic_annotation_target_for_parser_annotation(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    ast_annotation: &ast::Annotation,
    annotation_span: Span,
    target_id: u32,
) -> SemanticAnnotationTarget {
    match ast_annotation {
        ast::Annotation::Doc { .. } => resolve_doc_annotation_target(
            file,
            tree,
            tokens,
            parents,
            owner_index,
            annotation_span,
            target_id,
        ),
        ast::Annotation::Decorator { .. } => {
            let target_node_id = resolve_decorator_annotation_target(
                tree,
                tokens,
                parents,
                owner_index,
                annotation_span,
                target_id,
            );
            SemanticAnnotationTarget {
                target_node_id,
                doc_position_override: None,
            }
        }
    }
}

/// Project one parser semantic annotation into formatter projection storage.
fn project_parser_semantic_annotation(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    storage: &mut AnnotationProjectionStorage,
    target_id: u32,
    annotation_id: LocalNodeId<ast::Annotation>,
) {
    let ast_annotation = tree.get(annotation_id);
    let annotation_span = tree.get_span(annotation_id);
    let target = semantic_annotation_target_for_parser_annotation(
        file,
        tree,
        tokens,
        parents,
        owner_index,
        ast_annotation,
        annotation_span,
        target_id,
    );
    let annotation =
        semantic_formatter_annotation(file, tree, parents, ast_annotation, annotation_span, target);

    storage.push_entry(target.target_node_id, annotation, annotation_span);
}

/// Project parser side semantic annotations into formatter annotation entries.
fn project_parser_semantic_annotations(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    storage: &mut AnnotationProjectionStorage,
) {
    // add parser semantic annotations first
    for (&target_id, annotation_ids) in tree.get_all_annotations() {
        if !storage.has_target_node(target_id) {
            continue;
        }

        for &annotation_id in annotation_ids {
            project_parser_semantic_annotation(
                file,
                tree,
                tokens,
                parents,
                owner_index,
                storage,
                target_id,
                annotation_id,
            );
        }
    }
}

/// Project comment trivia into formatter annotation entries and return target spans.
fn project_comment_trivia_annotations(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    storage: &mut AnnotationProjectionStorage,
) -> Vec<(Span, u32)> {
    let mut comment_targets = Vec::<(Span, u32)>::new();

    // add comment trivia with formatter-side placement resolution
    for trivia in tree.comment_trivia().iter().copied() {
        let (target_id, position) = comment_trivia_attachment(
            file,
            tree,
            tokens,
            token_keyword_by_span,
            trivia,
            owner_index,
            parents,
        );

        let Some(target_id) = target_id else {
            continue;
        };
        if !push_comment_trivia_projection_entry(storage, target_id, trivia, position) {
            continue;
        }

        comment_targets.push((trivia.span, target_id));
    }

    comment_targets
}

/// Return the nearest comment target after one span.
fn next_comment_target_after_span(
    comment_targets: &[(Span, u32)],
    span: Span,
) -> Option<(Span, u32)> {
    comment_targets
        .iter()
        .find(|(comment_span, _)| comment_span.start >= span.end)
        .copied()
}

/// Return the nearest comment target before one span.
fn previous_comment_target_before_span(
    comment_targets: &[(Span, u32)],
    span: Span,
) -> Option<(Span, u32)> {
    comment_targets
        .iter()
        .rev()
        .find(|(comment_span, _)| comment_span.end <= span.start)
        .copied()
}

/// Return the first non-whitespace token type after one comment span.
fn first_non_whitespace_token_type_after_span(
    tokens: &[TokenSpan],
    span: Span,
) -> Option<TokenType> {
    let token_index = tokens.partition_point(|token| token.span.start < span.end);
    tokens.iter().skip(token_index).find_map(|token| {
        let token_type = token.token.ty;
        (!matches!(token_type, TokenType::Whitespace | TokenType::Newline)).then_some(token_type)
    })
}

/// Return whether one blank trivia span should be skipped near semicolon-guard comment boundaries.
fn blank_trivia_should_skip_semicolon_guard_boundary(
    tokens: &[TokenSpan],
    trivia_span: Span,
    token_before_is_statement_end: bool,
    token_before_is_if_without_else: bool,
    next_comment: Option<(Span, u32)>,
) -> bool {
    let next_comment_starts_semicolon_guard_boundary = next_comment
        .and_then(|(comment_span, _)| {
            first_non_whitespace_token_type_after_span(tokens, comment_span)
        })
        .is_some_and(|token_type| {
            matches!(
                token_type,
                TokenType::OpenBracket | TokenType::OpenParenthesis
            )
        });
    if !(token_before_is_statement_end
        && next_comment_starts_semicolon_guard_boundary
        && !token_before_is_if_without_else)
    {
        return false;
    }

    let Some((next_comment_span, _)) = next_comment else {
        return false;
    };
    if trivia_span.end > next_comment_span.start {
        return false;
    }

    token_range_is_whitespace_trivia_only(tokens, trivia_span.end, next_comment_span.start)
}

/// Return one blank-trivia fallback target when neighboring comments share one owner.
fn blank_trivia_neighbor_comment_bridge_target(
    previous_comment: Option<(Span, u32)>,
    next_comment: Option<(Span, u32)>,
) -> Option<u32> {
    let (Some((_, previous_comment_target)), Some((_, next_comment_target))) =
        (previous_comment, next_comment)
    else {
        return None;
    };
    if previous_comment_target != next_comment_target {
        return None;
    }

    Some(next_comment_target)
}

/// Token boundary facts for one blank-trivia projection step.
struct BlankTriviaProjectionFacts {
    /// The decoded token index after the blank trivia.
    token_after_index: Option<usize>,
    /// The token type after the blank trivia.
    token_after_type: Option<TokenType>,
    /// Whether the token before the blank seam ends one statement-like boundary.
    token_before_is_statement_end: bool,
    /// Whether the token before the blank seam belongs to `if (...)` without `else`.
    token_before_is_if_without_else: bool,
}

/// Return the semantic token type for one optional token index.
fn token_type_for_index(tokens: &[TokenSpan], token_index: Option<usize>) -> Option<TokenType> {
    token_index
        .and_then(|index| tokens.get(index))
        .map(|token| token.token.ty)
}

/// Return whether one token before blank trivia belongs to `if (...)` without `else`.
fn token_before_is_if_without_else(
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    token_before_index: Option<usize>,
) -> bool {
    token_before_index
        .and_then(|index| tokens.get(index))
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        .and_then(|owner| {
            if tree.get_node_type(owner) == NodeType::Expression {
                Some(owner)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Expression)
            }
        })
        .is_some_and(|owner| {
            let expression_id = LocalNodeId::<Expression>::new(owner);
            matches!(
                tree.get(expression_id),
                Expression::If {
                    else_expression: None,
                    ..
                }
            )
        })
}

/// Build projection facts for one blank-trivia seam.
fn blank_trivia_projection_facts(
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    trivia: ast::BlankTrivia,
) -> BlankTriviaProjectionFacts {
    // token boundary indexes and token types
    let token_before_index = decode_token_index(trivia.boundary.token_before);
    let token_after_index = decode_token_index(trivia.boundary.token_after);
    let token_before_type = token_type_for_index(tokens, token_before_index);
    let token_after_type = token_type_for_index(tokens, token_after_index);

    // statement-end token kinds relevant to semicolon-guard seams
    let token_before_is_statement_end = matches!(
        token_before_type,
        Some(TokenType::Semicolon | TokenType::CloseBrace | TokenType::CloseParenthesis)
    );

    // `if (...)` without else suppresses some semicolon-guard blank seams
    let token_before_is_if_without_else =
        token_before_is_if_without_else(tree, tokens, parents, token_before_index);

    BlankTriviaProjectionFacts {
        token_after_index,
        token_after_type,
        token_before_is_statement_end,
        token_before_is_if_without_else,
    }
}

/// Return whether one blank-trivia seam should be skipped at end-of-file.
fn blank_trivia_is_eof_seam(facts: &BlankTriviaProjectionFacts) -> bool {
    facts.token_after_index.is_none() || facts.token_after_type == Some(TokenType::End)
}

/// Resolve one blank trivia target from direct seam attachment and comment bridging fallback.
fn resolve_blank_trivia_target(
    tree: &NodeTree,
    tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    owner_index: &FormatterTriviaOwnerIndex,
    seam_index: &FormatterTriviaSeamIndex,
    parents: &NodeParentIndex,
    trivia: ast::BlankTrivia,
    previous_comment: Option<(Span, u32)>,
    next_comment: Option<(Span, u32)>,
) -> Option<(u32, AnnotationPosition)> {
    let (target_id, position) = blank_trivia_attachment(
        tree,
        tokens,
        token_keyword_by_span,
        trivia,
        owner_index,
        seam_index,
        parents,
    );
    if let Some(target_id) = target_id {
        return Some((target_id, position));
    }

    let comment_target =
        blank_trivia_neighbor_comment_bridge_target(previous_comment, next_comment)?;
    Some((comment_target, ast::AnnotationPosition::BlockPrefix))
}

/// Project blank trivia into formatter annotation entries.
fn project_blank_trivia_annotations(
    tree: &NodeTree,
    tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    owner_index: &FormatterTriviaOwnerIndex,
    seam_index: &FormatterTriviaSeamIndex,
    parents: &NodeParentIndex,
    comment_targets: &[(Span, u32)],
    storage: &mut AnnotationProjectionStorage,
) {
    // add blank trivia with formatter-side placement resolution
    for trivia in tree.blank_trivia().iter().copied() {
        let facts = blank_trivia_projection_facts(tree, tokens, parents, trivia);

        // normalize eof seams to one trailing newline
        if blank_trivia_is_eof_seam(&facts) {
            continue;
        }

        let next_comment = next_comment_target_after_span(comment_targets, trivia.span);
        if blank_trivia_should_skip_semicolon_guard_boundary(
            tokens,
            trivia.span,
            facts.token_before_is_statement_end,
            facts.token_before_is_if_without_else,
            next_comment,
        ) {
            continue;
        }

        let previous_comment = previous_comment_target_before_span(comment_targets, trivia.span);
        let Some((target_id, position)) = resolve_blank_trivia_target(
            tree,
            tokens,
            token_keyword_by_span,
            owner_index,
            seam_index,
            parents,
            trivia,
            previous_comment,
            next_comment,
        ) else {
            continue;
        };

        if !push_blank_trivia_projection_entry(storage, target_id, trivia, position) {
            continue;
        }
    }
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
    let mut storage = AnnotationProjectionStorage::new(node_count);
    let owner_index = formatter_trivia_owner_index(tree, tokens);

    // add parser semantic annotations first
    project_parser_semantic_annotations(file, tree, tokens, parents, &owner_index, &mut storage);

    // build formatter-side seam indexes for trivia placement
    let seam_index = formatter_trivia_seam_index(tree);

    // add comment trivia with formatter-side placement resolution
    let comment_targets = project_comment_trivia_annotations(
        file,
        tree,
        tokens,
        token_keyword_by_span,
        &owner_index,
        parents,
        &mut storage,
    );

    // add blank trivia with formatter-side placement resolution
    project_blank_trivia_annotations(
        tree,
        tokens,
        token_keyword_by_span,
        &owner_index,
        &seam_index,
        parents,
        &comment_targets,
        &mut storage,
    );

    // keep node-local annotation order source-stable
    storage.sort_by_source();

    storage.into_parts()
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
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.comment_is_multiline_star || !seam.has_trailing_newline || seam.has_leading_newline {
        return None;
    }

    let (tree, parents) = (ctx.tree, ctx.parents);
    if seam.token_before_is(TokenType::OpenBrace)
        && !seam.token_after_is(TokenType::CloseBrace)
        && let Some(target_node) = fallback_following_owner(ctx, owners)
    {
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    let preceding_owner_from_token_before = ctx
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let preceding_owner = preceding_owner_from_token_before.or(owners.preceding);
    let token_before_span = ctx.token_before_span.map(|token| token.span);

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
    ctx: &CommentSeamContext<'_>,
) -> Option<CommentAttachment> {
    if ctx.token_before.is_some() || ctx.token_after.is_some() {
        return None;
    }

    let target_node =
        find_smallest_owner_enclosing_range(ctx.tree, ctx.trivia.span.start, ctx.trivia.span.end)?;
    let target_node = normalize_formatter_trivia_target_owner(ctx.tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockPrefix))
}

/// Return one following owner normalized to the next token start.
fn fallback_following_owner(
    ctx: &CommentSeamContext<'_>,
    owners: CommentAttachmentNeighbors,
) -> Option<u32> {
    let target_node = following_owner_with_token_after_fallback(ctx.tree, ctx, owners.following)?;
    let token_after_span = ctx.token_after_span.map(|token| token.span);
    let target_node = token_after_span
        .map(|span| promote_owner_by_shared_start(ctx.tree, ctx.parents, target_node, span.start))
        .unwrap_or(target_node);
    Some(normalize_formatter_trivia_target_owner(
        ctx.tree,
        target_node,
    ))
}

/// Return one preceding owner normalized to the previous token end.
fn fallback_preceding_owner(
    ctx: &CommentSeamContext<'_>,
    owners: CommentAttachmentNeighbors,
) -> Option<u32> {
    let target_node = owners.preceding.or_else(|| {
        ctx.token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(ctx.tree, token.span))
    })?;
    let token_before_span = ctx.token_before_span.map(|token| token.span);
    Some(normalize_owner_with_shared_end(
        ctx.tree,
        ctx.parents,
        target_node,
        token_before_span,
    ))
}

/// Attach one fallback-following owner with one target position.
#[inline]
fn attach_to_following_owner(
    ctx: &CommentSeamContext<'_>,
    owners: CommentAttachmentNeighbors,
    position: AnnotationPosition,
) -> Option<CommentAttachment> {
    let target_node = fallback_following_owner(ctx, owners)?;
    Some((Some(target_node), position))
}

/// Attach one fallback-preceding owner with one target position.
#[inline]
fn attach_to_preceding_owner(
    ctx: &CommentSeamContext<'_>,
    owners: CommentAttachmentNeighbors,
    position: AnnotationPosition,
) -> Option<CommentAttachment> {
    let target_node = fallback_preceding_owner(ctx, owners)?;
    Some((Some(target_node), position))
}

/// Attach one enclosing owner as block infix when no neighbor fallback applies.
fn attach_to_enclosing_owner_infix(
    ctx: &CommentSeamContext<'_>,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
) -> Option<CommentAttachment> {
    let enclosing_owner = comment_enclosing_owner(ctx, enclosing_owner_cache)?;
    let enclosing_owner = normalize_formatter_trivia_target_owner(ctx.tree, enclosing_owner);
    Some((Some(enclosing_owner), AnnotationPosition::BlockInfix))
}

/// Return one trailing expression owner that shares one seam-end token.
fn trailing_expression_owner_for_seam_end(
    tree: &NodeTree,
    mut target_owner: u32,
    seam_end: u32,
) -> u32 {
    loop {
        if tree.get_node_type(target_owner) != NodeType::Expression {
            break;
        }

        let expression_id = LocalNodeId::<Expression>::new(target_owner);
        let next_owner = match tree.get(expression_id) {
            Expression::Binary { right, .. } | Expression::TypeBinary { right, .. } => {
                Some(right.id)
            }
            Expression::Parenthesized { expression } => Some(expression.id),
            _ => None,
        };

        let Some(next_owner) = next_owner else {
            break;
        };
        if tree.get_span_by_id(next_owner).end != seam_end {
            break;
        }

        target_owner = next_owner;
    }

    target_owner
}

/// Attach one preceding owner for end-of-line fallback with terminal-owner normalization rules.
fn attach_end_of_line_preceding_owner(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let (tree, parents) = (ctx.tree, ctx.parents);
    let target_node = owners.preceding?;
    let token_before_span = ctx.token_before_span.map(|token| token.span);

    // trailing line comments before `)` stay on the trailing expression operand
    if seam.comment_is_line && seam.token_after_is(TokenType::CloseParenthesis) {
        let target_node = token_before_span.map_or(target_node, |span| {
            trailing_expression_owner_for_seam_end(tree, target_node, span.end)
        });
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // trailing block comments between `}` and `)` are callback/body postfix seams
    if seam.comment_is_star
        && seam.token_before_is(TokenType::CloseBrace)
        && seam.token_after_is(TokenType::CloseParenthesis)
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
        return Some((Some(target_node), AnnotationPosition::BlockPostfix));
    }

    let target_node =
        normalize_owner_with_shared_end(tree, parents, target_node, token_before_span);
    let position = AnnotationPosition::LinePostfixBoundary;

    Some((Some(target_node), position))
}

/// Return whether one seam comment is an ignore directive line comment.
fn seam_comment_is_ignore_directive(ctx: &CommentSeamContext<'_>, seam: &CommentSeamData) -> bool {
    seam.comment_is_line && comment_directive_is_ignore(ctx.trivia.directive)
}

/// Resolve own-line ignore-directive comments to one line-prefix target.
fn attach_default_own_line_ignore_directive_comment(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam_comment_is_ignore_directive(ctx, seam) {
        return None;
    }

    let following_owner =
        following_owner_with_token_after_fallback(ctx.tree, ctx, owners.following);
    if let Some(target_node) = following_owner {
        let target_node = normalize_formatter_trivia_target_owner(ctx.tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    attach_to_following_owner(
        ctx,
        CommentAttachmentNeighbors {
            preceding: owners.preceding,
            following: following_owner,
        },
        AnnotationPosition::LinePrefix,
    )
}

/// Return following placement for one own-line default seam.
fn own_line_default_following_position(seam: &CommentSeamData) -> AnnotationPosition {
    if seam.comment_is_line {
        return AnnotationPosition::LinePrefix;
    }

    AnnotationPosition::BlockPrefix
}

/// Return preceding placement for one own-line default seam.
fn own_line_default_preceding_position(seam: &CommentSeamData) -> AnnotationPosition {
    if seam.comment_is_line {
        return AnnotationPosition::LinePostfixBoundary;
    }

    AnnotationPosition::BlockPostfix
}

/// Attach one own-line comment with one canonical placement fallback.
fn attach_default_own_line_comment(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    // ignore directives stay line-prefix and prefer following ownership
    if let Some(attachment) = attach_default_own_line_ignore_directive_comment(ctx, seam, owners) {
        return Some(attachment);
    }

    // following side owns own-line comments by default
    let resolved_owners = CommentAttachmentNeighbors {
        preceding: owners.preceding,
        following: following_owner_with_token_after_fallback(ctx.tree, ctx, owners.following),
    };
    let following_position = own_line_default_following_position(seam);
    if let Some(attachment) = attach_to_following_owner(ctx, resolved_owners, following_position) {
        return Some(attachment);
    }

    // preceding side owns when following cannot be resolved
    let preceding_position = own_line_default_preceding_position(seam);
    if let Some(attachment) = attach_to_preceding_owner(ctx, owners, preceding_position) {
        return Some(attachment);
    }

    // fall back to enclosing owner when neither neighboring owner resolves
    attach_to_enclosing_owner_infix(ctx, enclosing_owner_cache)
}

/// Attach one end-of-line comment with one canonical placement fallback.
fn attach_default_end_of_line_comment(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if let Some(attachment) = attach_end_of_line_preceding_owner(ctx, seam, owners) {
        return Some(attachment);
    }

    if let Some(attachment) = attach_to_following_owner(ctx, owners, AnnotationPosition::LinePrefix)
    {
        return Some(attachment);
    }

    attach_to_enclosing_owner_infix(ctx, enclosing_owner_cache)
}

/// Attach one remaining comment with one canonical placement fallback.
fn attach_default_remaining_comment(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    // separators and operators that bind to the left keep trailing ownership
    if seam.token_after_prefers_preceding {
        return attach_to_preceding_owner(ctx, owners, AnnotationPosition::LinePostfix);
    }

    // right-binding seams keep prefix ownership
    if seam.seam_binds_right {
        return attach_to_following_owner(ctx, owners, AnnotationPosition::LinePrefix);
    }

    // default stable ownership: prefer preceding before following
    if let Some(attachment) =
        attach_to_preceding_owner(ctx, owners, AnnotationPosition::LinePostfix)
    {
        return Some(attachment);
    }
    if let Some(attachment) = attach_to_following_owner(ctx, owners, AnnotationPosition::LinePrefix)
    {
        return Some(attachment);
    }

    attach_to_enclosing_owner_infix(ctx, enclosing_owner_cache)
}

/// Resolve own-line placement using specialized routing then default ownership.
fn attach_own_line_comment_with_default(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if let Some(attachment) = attach_own_line_comment(ctx, seam, enclosing_owner_cache, owners) {
        return Some(attachment);
    }

    attach_default_own_line_comment(ctx, seam, enclosing_owner_cache, owners)
}

/// Resolve end-of-line placement using specialized routing then default ownership.
fn attach_end_of_line_comment_with_default(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if let Some(attachment) =
        try_attach_multiline_inline_block_comment_before_line_end(ctx, seam, owners)
    {
        return Some(attachment);
    }

    if let Some(attachment) = attach_end_of_line_comment(ctx, seam, enclosing_owner_cache, owners) {
        return Some(attachment);
    }

    attach_default_end_of_line_comment(ctx, seam, enclosing_owner_cache, owners)
}

/// Resolve remaining placement using specialized routing then default ownership.
fn attach_remaining_comment_with_default(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if let Some(attachment) = attach_remaining_comment(ctx, seam, owners) {
        return Some(attachment);
    }

    attach_default_remaining_comment(ctx, seam, enclosing_owner_cache, owners)
}

/// Resolve the default comment trivia rules after specialized seam cases.
pub(crate) fn attach_comment_default(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> CommentAttachment {
    let placement = classify_comment_placement(ctx, seam);

    // placement route
    let placement_attachment = match placement {
        CommentPlacement::OwnLine => {
            attach_own_line_comment_with_default(ctx, seam, enclosing_owner_cache, owners)
        }
        CommentPlacement::EndOfLine => {
            attach_end_of_line_comment_with_default(ctx, seam, enclosing_owner_cache, owners)
        }
        CommentPlacement::Remaining => {
            attach_remaining_comment_with_default(ctx, seam, enclosing_owner_cache, owners)
        }
    };
    if let Some(attachment) = placement_attachment {
        return attachment;
    }

    // comment only files can still anchor to one enclosing owner
    if let Some(attachment) = try_attach_comment_only_file_fallback(ctx) {
        return attachment;
    }

    (None, AnnotationPosition::BlockInfix)
}

/// Resolve one following owner with one token-after fallback owner.
pub(crate) fn following_owner_with_token_after_fallback(
    tree: &NodeTree,
    ctx: &CommentSeamContext<'_>,
    following_owner: Option<u32>,
) -> Option<u32> {
    following_owner
        .or_else(|| {
            ctx.token_after.and_then(|token_index| {
                find_owner_at_or_after_token(tree, ctx.semantic_tokens, token_index)
            })
        })
        .or_else(|| {
            ctx.token_after_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        })
}

/// Resolve one preceding owner with one token-before fallback owner.
pub(crate) fn preceding_owner_with_token_before_fallback(
    tree: &NodeTree,
    ctx: &CommentSeamContext<'_>,
    preceding_owner: Option<u32>,
) -> Option<u32> {
    preceding_owner.or_else(|| {
        ctx.token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
    })
}

/// Resolve one preceding owner with one previous non-newline token fallback owner.
pub(crate) fn preceding_owner_with_non_newline_token_before_fallback(
    tree: &NodeTree,
    ctx: &CommentSeamContext<'_>,
    preceding_owner: Option<u32>,
) -> Option<u32> {
    let fallback_owner = ctx
        .token_before
        .and_then(|token_index| previous_non_newline_token_index(ctx.semantic_tokens, token_index))
        .and_then(|token_index| ctx.semantic_tokens.get(token_index))
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));

    preceding_owner.or(fallback_owner)
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
