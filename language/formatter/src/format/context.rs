use std::borrow::Cow;
use std::cell::{Cell, OnceCell, Ref, RefCell};
use std::rc::Rc;

use ast::{
    AnnotationPosition, Argument, Blank, Block, Comment, Declaration, Declarator, Decorator,
    DependencyItem, Doc, EnumField, Expression, LocalNodeId, LocalNodeIdAny, MatchCase, Member,
    Node, NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern, PatternField,
    Property, TokenSpan, TokenType, WhereClause, normalize_comment_payload,
};
use destack_ast as ast;
use destack_base::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter, GroupId};
use destack_fir::print::PrintOptions;
use destack_source::{File, IndentStyle, LanguageType, LineEnding, MultiSpan, NodeSourceMap, Span};
use destack_workspace::{
    ArrowParentheses, FormatterOptions, ImportSortOrder, OrganizeImports, QuoteProperty,
    QuoteStyle, TrailingComma,
};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use super::timing::{
    FormatterTimingEntry, FormatterTimingScope, FormatterTimingTag, FormatterTimings,
    tag_for_node_type, timings_enabled_from_env,
};

const ANNOTATION_STATE_NONE: u8 = 1;
const ANNOTATION_STATE_PRESENT: u8 = 2;
const ANNOTATION_STATE_CACHED: u8 = 3;
const NODE_BOOL_STATE_UNKNOWN: u8 = 0;
const NODE_BOOL_STATE_FALSE: u8 = 1;
const NODE_BOOL_STATE_TRUE: u8 = 2;
const NODE_SPAN_CHAR_LEN_UNKNOWN: u32 = u32::MAX;
const TYPE_CONTEXT_STATE_UNKNOWN: u8 = 0;
const TYPE_CONTEXT_STATE_FALSE: u8 = 1;
const TYPE_CONTEXT_STATE_TRUE: u8 = 2;
const NO_TOKEN_INDEX: u32 = u32::MAX;

/// Formatter-owned annotation payload for semantic annotations and placed trivia.
#[derive(Debug, Clone, Copy)]
pub enum Annotation {
    /// Blank trivia annotation.
    Blank {
        /// The blank node.
        node: LocalNodeId<Blank>,
        /// The resolved annotation position.
        position: AnnotationPosition,
    },
    /// Documentation annotation.
    Doc {
        /// The doc node.
        node: LocalNodeId<Doc>,
        /// The resolved annotation position.
        position: AnnotationPosition,
    },
    /// Comment trivia annotation.
    Comment {
        /// The comment node.
        node: LocalNodeId<Comment>,
        /// The resolved annotation position.
        position: AnnotationPosition,
    },
    /// Decorator annotation.
    Decorator {
        /// The decorator node.
        node: LocalNodeId<Decorator>,
        /// The resolved annotation position.
        position: AnnotationPosition,
    },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;
}

impl Annotation {
    /// Return the annotation position.
    #[inline]
    pub fn position(self) -> AnnotationPosition {
        match self {
            Annotation::Blank { position, .. } => position,
            Annotation::Doc { position, .. } => position,
            Annotation::Comment { position, .. } => position,
            Annotation::Decorator { position, .. } => position,
        }
    }
}

/// Formatter-local annotation entry with resolved span.
#[derive(Debug, Clone, Copy)]
pub struct FormatterAnnotationEntry {
    /// The annotation payload and position.
    pub annotation: Annotation,
    /// The annotation span.
    pub span: Span,
}

pub type DestackFormatter<'ast, 'buf> = Formatter<'buf, DestackFormatContext<'ast>>;

/// Snapshot of formatter cache behavior counters.
#[derive(Debug, Clone, Copy, Default)]
pub struct FormatterCacheStatsSnapshot {
    /// Number of `span_text_cache` hits.
    pub span_text_hits: usize,
    /// Number of `span_text_cache` misses.
    pub span_text_misses: usize,
    /// Number of `span_has_newline_cache` hits.
    pub span_has_newline_hits: usize,
    /// Number of `span_has_newline_cache` misses.
    pub span_has_newline_misses: usize,
    /// Number of `span_has_comment_cache` hits.
    pub span_has_comment_hits: usize,
    /// Number of `span_has_comment_cache` misses.
    pub span_has_comment_misses: usize,
    /// Number of annotation cache hits.
    pub annotation_cache_hits: usize,
    /// Number of annotation cache misses.
    pub annotation_cache_misses: usize,
}

/// Shared formatter cache instrumentation counters.
#[derive(Debug, Default)]
pub struct FormatterCacheStatsCollector {
    /// Number of `span_text_cache` hits.
    pub span_text_hits: Cell<usize>,
    /// Number of `span_text_cache` misses.
    pub span_text_misses: Cell<usize>,
    /// Number of `span_has_newline_cache` hits.
    pub span_has_newline_hits: Cell<usize>,
    /// Number of `span_has_newline_cache` misses.
    pub span_has_newline_misses: Cell<usize>,
    /// Number of `span_has_comment_cache` hits.
    pub span_has_comment_hits: Cell<usize>,
    /// Number of `span_has_comment_cache` misses.
    pub span_has_comment_misses: Cell<usize>,
    /// Number of annotation cache hits.
    pub annotation_cache_hits: Cell<usize>,
    /// Number of annotation cache misses.
    pub annotation_cache_misses: Cell<usize>,
}

/// One formatter instrumentation counter entry.
#[derive(Debug, Clone, Copy)]
pub struct FormatterCounterEntry {
    /// The counter name.
    pub name: &'static str,
    /// The counter value.
    pub value: usize,
}

/// Shared formatter instrumentation counters.
#[derive(Debug, Default)]
pub struct FormatterCountersCollector {
    /// Counter values keyed by static counter name.
    pub counters: RefCell<FxHashMap<&'static str, usize>>,
}

impl FormatterCountersCollector {
    /// Increment a counter by a delta.
    pub fn increment(&self, name: &'static str, delta: usize) {
        let mut counters = self.counters.borrow_mut();
        let value = counters.entry(name).or_insert(0);
        *value = value.saturating_add(delta);
    }

    /// Snapshot all counters sorted by name.
    pub fn snapshot(&self) -> Vec<FormatterCounterEntry> {
        let counters = self.counters.borrow();
        let mut snapshot = counters
            .iter()
            .map(|(name, value)| FormatterCounterEntry {
                name,
                value: *value,
            })
            .collect::<Vec<_>>();
        snapshot.sort_by_key(|entry| entry.name);
        snapshot
    }
}

/// Cached annotation data for one node.
#[derive(Debug, Clone)]
pub struct CachedAnnotationData {
    /// Annotation ids attached to the node.
    pub ids: Vec<LocalNodeId<Annotation>>,
    /// Whether any annotation is non-blank.
    pub has_non_blank: bool,
    /// Whether any annotation is a prefix annotation.
    pub has_prefix: bool,
    /// Whether any annotation is an infix annotation.
    pub has_infix: bool,
    /// Whether any annotation is a non-blank infix annotation.
    pub has_non_blank_infix: bool,
    /// Whether any annotation is a postfix annotation.
    pub has_postfix: bool,
    /// Whether any annotation is a blank prefix annotation.
    pub has_blank_prefix: bool,
    /// Whether the first annotation is a blank prefix annotation.
    pub has_blank_prefix_first: bool,
}

impl CachedAnnotationData {
    /// Build cached annotation metadata from annotation ids.
    pub fn from_ids(
        ids: Vec<LocalNodeId<Annotation>>,
        mut annotation_for: impl FnMut(LocalNodeId<Annotation>) -> Annotation,
    ) -> Self {
        let mut has_non_blank = false;
        let mut has_prefix = false;
        let mut has_infix = false;
        let mut has_non_blank_infix = false;
        let mut has_postfix = false;
        let mut has_blank_prefix = false;
        let mut has_blank_prefix_first = false;

        for (index, annotation_id) in ids.iter().enumerate() {
            let annotation = annotation_for(*annotation_id);
            let position = annotation.position();
            let is_non_blank = !matches!(annotation, Annotation::Blank { .. });

            if is_non_blank {
                has_non_blank = true;
            }

            // general position flags
            if position == AnnotationPosition::BlockPrefix
                || position == AnnotationPosition::LinePrefix
            {
                has_prefix = true;
            }

            if position == AnnotationPosition::BlockInfix {
                has_infix = true;
                if is_non_blank {
                    has_non_blank_infix = true;
                }
            }

            if position == AnnotationPosition::BlockPostfix
                || position == AnnotationPosition::LinePostfix
                || position == AnnotationPosition::LinePostfixBoundary
            {
                has_postfix = true;
            }

            // blank prefix flags
            if matches!(annotation, Annotation::Blank { .. })
                && (position == AnnotationPosition::BlockPrefix
                    || position == AnnotationPosition::LinePrefix)
            {
                has_blank_prefix = true;
                if index == 0 {
                    has_blank_prefix_first = true;
                }
            }
        }

        Self {
            ids,
            has_non_blank,
            has_prefix,
            has_infix,
            has_non_blank_infix,
            has_postfix,
            has_blank_prefix,
            has_blank_prefix_first,
        }
    }
}

/// Build formatter-owned annotation ids and entries from semantic attachments and trivia.
#[derive(Debug)]
struct FormatterTokenNeighborIndex {
    previous_attachable: Vec<Option<usize>>,
    next_attachable: Vec<Option<usize>>,
}

#[derive(Debug)]
struct FormatterTriviaOwnerIndex {
    owner_start_by_token: Vec<Option<u32>>,
    owner_end_by_token: Vec<Option<u32>>,
    nearest_owner_start_by_token: Vec<Option<u32>>,
    nearest_owner_end_by_token: Vec<Option<u32>>,
}

#[derive(Debug)]
struct FormatterTriviaSeamIndex {
    line_comment_seams: FxHashSet<u64>,
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

/// Return whether one node kind is excluded from trivia owner indexing.
fn is_trivia_excluded_owner_node_id(tree: &NodeTree, node_id: u32) -> bool {
    matches!(
        tree.get_node_type(node_id),
        NodeType::Annotation
            | NodeType::Doc
            | NodeType::Comment
            | NodeType::Blank
            | NodeType::Decorator
    )
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
fn build_formatter_trivia_owner_index(
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
fn encode_trivia_seam(token_before: u32, token_after: u32) -> u64 {
    ((token_before as u64) << 32) | token_after as u64
}

/// Build formatter-side trivia seam indexes.
fn build_formatter_trivia_seam_index(tree: &NodeTree) -> FormatterTriviaSeamIndex {
    let mut line_comment_seams = FxHashSet::default();
    line_comment_seams.reserve(tree.comment_trivia().len());

    // cache line-comment seam keys once for blank attachment checks
    for trivia in tree.comment_trivia().iter().copied() {
        if tree.get(trivia.comment).style == ast::CommentStyle::Slash {
            let seam =
                encode_trivia_seam(trivia.boundary.token_before, trivia.boundary.token_after);
            line_comment_seams.insert(seam);
        }
    }

    FormatterTriviaSeamIndex { line_comment_seams }
}

/// Decode one compact token index with sentinel for none.
fn decode_token_index(token_index: u32) -> Option<usize> {
    (token_index != NO_TOKEN_INDEX).then_some(token_index as usize)
}

/// Normalize one trivia owner id to the canonical structural owner.
fn normalize_formatter_trivia_target_owner(tree: &NodeTree, owner_id: u32) -> u32 {
    let mut current_id = owner_id;

    loop {
        if tree.get_node_type(current_id) != NodeType::Expression {
            return current_id;
        }

        let expression_id = LocalNodeId::<Expression>::new(current_id);
        match tree.get(expression_id) {
            // statement wrappers are transparent for trivia ownership
            Expression::Statement(expression) => {
                current_id = expression.id;
            }
            // parenthesized wrappers are transparent for trivia ownership
            Expression::Parenthesized { expression } => {
                current_id = expression.id;
            }
            // declaration expression trivia belongs to the declaration node
            Expression::Declaration(declaration) => {
                return declaration.id;
            }
            _ => {
                return current_id;
            }
        }
    }
}

/// Return whether one token kind is an opening delimiter.
#[inline]
fn is_open_delimiter_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
    )
}

/// Return whether one token kind is a closing delimiter.
#[inline]
fn is_close_delimiter_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::CloseParenthesis | TokenType::CloseBrace | TokenType::CloseBracket
    )
}

/// Return whether one open and close delimiter token pair matches.
#[inline]
fn delimiters_match(open: TokenType, close: TokenType) -> bool {
    matches!(
        (open, close),
        (TokenType::OpenParenthesis, TokenType::CloseParenthesis)
            | (TokenType::OpenBrace, TokenType::CloseBrace)
            | (TokenType::OpenBracket, TokenType::CloseBracket)
    )
}

/// Return whether one token after a comment seam prefers left ownership.
#[inline]
fn token_after_prefers_left_ownership(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::Semicolon
            | TokenType::Comma
            | TokenType::CloseParenthesis
            | TokenType::CloseBrace
            | TokenType::CloseBracket
            | TokenType::Maybe
            | TokenType::ElementwiseAnd
            | TokenType::ElementwiseOr
            | TokenType::ElementwiseXor
            | TokenType::LogicalAnd
            | TokenType::LogicalOr
            | TokenType::Coalesce
            | TokenType::Equal
            | TokenType::EqualWide
            | TokenType::NotEqual
            | TokenType::NotEqualWide
            | TokenType::LessThan
            | TokenType::LessThanOrEqual
            | TokenType::GreaterThan
            | TokenType::GreaterThanOrEqual
            | TokenType::Add
            | TokenType::WrappingAdd
            | TokenType::SaturatingAdd
            | TokenType::Subtract
            | TokenType::WrappingSubtract
            | TokenType::SaturatingSubtract
            | TokenType::Multiply
            | TokenType::WrappingMultiply
            | TokenType::SaturatingMultiply
            | TokenType::Exponent
            | TokenType::WrappingExponent
            | TokenType::SaturatingExponent
            | TokenType::Divide
            | TokenType::Remainder
            | TokenType::ShiftLeft
            | TokenType::SaturatingShiftLeft
            | TokenType::Assign
    )
}

/// Return one preferred owner that starts at one token span.
fn find_preferred_owner_starting_at(tree: &NodeTree, span: Span) -> Option<u32> {
    let mut best_owner: Option<destack_source::EnclosingSpan> = None;
    tree.source_map
        .visit_enclosing_spans(span.start, span.end.saturating_sub(1), |candidate| {
            if candidate.span.start != span.start
                || is_trivia_excluded_owner_node_id(tree, candidate.idx)
            {
                return;
            }

            let candidate_kind_rank = if tree.get_node_type(candidate.idx) == NodeType::Expression {
                1
            } else {
                0
            };

            let should_replace = if let Some(current) = best_owner {
                let current_kind_rank = if tree.get_node_type(current.idx) == NodeType::Expression {
                    1
                } else {
                    0
                };
                candidate.length < current.length
                    || (candidate.length == current.length
                        && candidate_kind_rank < current_kind_rank)
                    || (candidate.length == current.length
                        && candidate_kind_rank == current_kind_rank
                        && candidate.idx < current.idx)
            } else {
                true
            };

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Return one smallest owner that encloses one token span.
fn find_smallest_owner_enclosing_token(tree: &NodeTree, span: Span) -> Option<u32> {
    let mut best_owner: Option<destack_source::EnclosingSpan> = None;
    tree.source_map
        .visit_enclosing_spans(span.start, span.end.saturating_sub(1), |candidate| {
            if is_trivia_excluded_owner_node_id(tree, candidate.idx) {
                return;
            }

            let should_replace = if let Some(current) = best_owner {
                candidate.length < current.length
                    || (candidate.length == current.length && candidate.idx < current.idx)
            } else {
                true
            };

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Return one smallest owner that encloses one seam range.
fn find_smallest_owner_enclosing_range(tree: &NodeTree, start: u32, end: u32) -> Option<u32> {
    if start >= end {
        return None;
    }

    let mut best_owner: Option<destack_source::EnclosingSpan> = None;
    tree.source_map
        .visit_enclosing_spans(start, end.saturating_sub(1), |candidate| {
            if is_trivia_excluded_owner_node_id(tree, candidate.idx) {
                return;
            }

            let should_replace = if let Some(current) = best_owner {
                candidate.length < current.length
                    || (candidate.length == current.length && candidate.idx < current.idx)
            } else {
                true
            };

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Promote one owner while ancestor spans share the same seam end.
fn promote_owner_by_shared_end(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    seam_end: u32,
) -> u32 {
    let mut current_id = owner_id;
    let mut best_id = owner_id;

    while let Some(parent_id) = parents.get_by_id(current_id) {
        let parent_span = tree.get_span_by_id(parent_id);
        if parent_span.end != seam_end {
            break;
        }
        if is_trivia_excluded_owner_node_id(tree, parent_id) {
            break;
        }

        best_id = parent_id;
        current_id = parent_id;
    }

    best_id
}

/// Promote one owner while ancestor spans share the same seam start.
fn promote_owner_by_shared_start(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    seam_start: u32,
) -> u32 {
    let mut current_id = owner_id;
    let mut best_id = owner_id;

    while let Some(parent_id) = parents.get_by_id(current_id) {
        let parent_span = tree.get_span_by_id(parent_id);
        if parent_span.start != seam_start {
            break;
        }
        if is_trivia_excluded_owner_node_id(tree, parent_id) {
            break;
        }

        best_id = parent_id;
        current_id = parent_id;
    }

    best_id
}

/// Return the previous non-newline semantic token index before one index.
fn previous_non_newline_token_index(semantic_tokens: &[TokenSpan], index: usize) -> Option<usize> {
    if index == 0 {
        return None;
    }

    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        if semantic_tokens[cursor].token.ty != TokenType::Newline {
            return Some(cursor);
        }
    }

    None
}

/// Return one block-interior placement target for boundary comments.
fn resolve_block_leading_comment_target(
    tree: &NodeTree,
    block_id: LocalNodeId<Block>,
) -> (u32, AnnotationPosition) {
    let block = tree.get(block_id);
    if let Some(first_expression) = block.expressions.first().copied() {
        return (first_expression.id, AnnotationPosition::BlockPrefix);
    }

    if block.format == ast::BlockFormat::Explicit {
        return (block_id.id, AnnotationPosition::BlockInfix);
    }

    (block_id.id, AnnotationPosition::BlockPrefix)
}

/// Promote one owner to the nearest declaration ancestor.
fn promote_owner_to_declaration_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Declaration {
            return Some(node_id);
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Promote one owner to the nearest ancestor of one node type.
fn promote_owner_to_node_type_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    node_type: NodeType,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == node_type {
            return Some(node_id);
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Promote one owner to the nearest `satisfies` expression ancestor.
fn promote_owner_to_satisfies_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if matches!(
                tree.get(expression_id),
                Expression::TypeBinary {
                    operator: ast::TypeBinaryOperator::Satisfies,
                    ..
                }
            ) {
                return Some(node_id);
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Find the next declaration owner at or after one semantic token index.
fn find_next_declaration_owner_from_token(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: usize,
) -> Option<u32> {
    let token_count = owner_index.owner_start_by_token.len();
    if token_index >= token_count {
        return None;
    }

    let search_end = (token_index + 96).min(token_count);
    for current_index in token_index..search_end {
        let candidate_owner = owner_index.owner_start_by_token[current_index]
            .or(owner_index.nearest_owner_start_by_token[current_index]);
        let Some(candidate_owner) = candidate_owner else {
            continue;
        };

        let candidate_owner = normalize_formatter_trivia_target_owner(tree, candidate_owner);
        if tree.get_node_type(candidate_owner) == NodeType::Declaration {
            return Some(candidate_owner);
        }
    }

    None
}

/// Find the next member owner at or after one semantic token index.
fn find_next_member_owner_from_token(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: usize,
) -> Option<u32> {
    let token_count = owner_index.owner_start_by_token.len();
    if token_index >= token_count {
        return None;
    }

    let search_end = (token_index + 96).min(token_count);
    for current_index in token_index..search_end {
        let candidate_owner = owner_index.owner_start_by_token[current_index]
            .or(owner_index.nearest_owner_start_by_token[current_index]);
        let Some(candidate_owner) = candidate_owner else {
            continue;
        };

        if tree.get_node_type(candidate_owner) == NodeType::Member {
            return Some(candidate_owner);
        }
    }

    None
}

/// Return one lowest common ancestor for two owners.
fn lowest_common_owner_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    left_owner: u32,
    right_owner: u32,
) -> Option<u32> {
    let mut left_chain = SmallVec::<[u32; 24]>::new();
    let mut current_left = Some(left_owner);
    while let Some(owner_id) = current_left {
        left_chain.push(owner_id);
        current_left = parents.get_by_id(owner_id);
    }

    let mut current_right = Some(right_owner);
    while let Some(owner_id) = current_right {
        if left_chain.contains(&owner_id) && !is_trivia_excluded_owner_node_id(tree, owner_id) {
            return Some(owner_id);
        }
        current_right = parents.get_by_id(owner_id);
    }

    None
}

/// Return whether one close parenthesis token ends a control-flow head.
fn token_is_control_head_close_paren(
    file: &File,
    semantic_tokens: &[TokenSpan],
    close_paren_index: usize,
) -> bool {
    if semantic_tokens
        .get(close_paren_index)
        .is_none_or(|token| token.token.ty != TokenType::CloseParenthesis)
    {
        return false;
    }

    let mut depth = 1usize;
    let mut cursor = close_paren_index;
    let mut open_paren_index = None;
    while cursor > 0 {
        cursor -= 1;
        let token = semantic_tokens[cursor];
        match token.token.ty {
            TokenType::CloseParenthesis => depth += 1,
            TokenType::OpenParenthesis => {
                depth -= 1;
                if depth == 0 {
                    open_paren_index = Some(cursor);
                    break;
                }
            }
            _ => {}
        }
    }

    let Some(open_paren_index) = open_paren_index else {
        return false;
    };
    if open_paren_index == 0 {
        return false;
    }

    let mut keyword_cursor = open_paren_index;
    while keyword_cursor > 0 {
        keyword_cursor -= 1;
        let token = semantic_tokens[keyword_cursor];
        if token.token.ty == TokenType::Newline {
            continue;
        }

        return token.token.ty == TokenType::Identifier
            && matches!(
                file.span_str(token.span),
                "if" | "for" | "while" | "catch" | "with"
            );
    }

    false
}

/// One normalized identifier keyword used in comment seam rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommentSeamKeyword {
    /// One non-keyword identifier.
    None,
    /// One `as` keyword.
    As,
    /// One `satisfies` keyword.
    Satisfies,
    /// One `export` keyword.
    Export,
    /// One `implements` keyword.
    Implements,
    /// One `else` keyword.
    Else,
    /// One `case` keyword.
    Case,
    /// One `default` keyword.
    Default,
    /// One `const` keyword.
    Const,
}

/// Classify one identifier token into one seam keyword family.
#[inline]
fn classify_comment_seam_keyword(file: &File, token: Option<TokenSpan>) -> CommentSeamKeyword {
    let Some(token) = token else {
        return CommentSeamKeyword::None;
    };

    if token.token.ty != TokenType::Identifier {
        return CommentSeamKeyword::None;
    }

    match file.span_str(token.span) {
        "as" => CommentSeamKeyword::As,
        "satisfies" => CommentSeamKeyword::Satisfies,
        "export" => CommentSeamKeyword::Export,
        "implements" => CommentSeamKeyword::Implements,
        "else" => CommentSeamKeyword::Else,
        "case" => CommentSeamKeyword::Case,
        "default" => CommentSeamKeyword::Default,
        "const" => CommentSeamKeyword::Const,
        _ => CommentSeamKeyword::None,
    }
}

/// Immutable context for one comment seam attachment decision.
#[derive(Clone, Copy)]
struct CommentSeamContext<'a> {
    /// The source file.
    file: &'a File,
    /// The syntax tree.
    tree: &'a NodeTree,
    /// The semantic token stream.
    semantic_tokens: &'a [TokenSpan],
    /// The comment trivia payload.
    trivia: destack_ast::CommentTrivia,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// Token index before the seam.
    token_before: Option<usize>,
    /// Token index after the seam.
    token_after: Option<usize>,
    /// Token span before the seam.
    token_before_span: Option<TokenSpan>,
    /// Token span after the seam.
    token_after_span: Option<TokenSpan>,
}

/// Compact seam facts derived once per comment seam.
#[derive(Clone, Copy)]
struct CommentSeamFacts {
    /// Whether trivia has at least one newline before comment text.
    has_leading_newline: bool,
    /// Whether trivia has at least one newline after comment text.
    has_trailing_newline: bool,
    /// Whether comment style is `//`.
    comment_is_line: bool,
    /// Whether comment style is `/* */`.
    comment_is_star: bool,
    /// Whether `/* */` comment text spans multiple lines.
    comment_is_multiline_star: bool,
    /// Token kind before seam.
    token_before_type: Option<TokenType>,
    /// Token kind after seam.
    token_after_type: Option<TokenType>,
    /// Keyword class for identifier before seam.
    token_before_keyword: CommentSeamKeyword,
    /// Keyword class for identifier after seam.
    token_after_keyword: CommentSeamKeyword,
    /// Whether token after seam structurally prefers left ownership.
    token_after_prefers_left: bool,
    /// Whether token before seam closes one control-flow head.
    token_before_is_control_head_close_paren: bool,
    /// Whether seam is one return type boundary after `):`.
    token_before_is_return_type_colon: bool,
    /// Whether default trailing behavior should prefer right binding.
    seam_binds_right: bool,
}

impl CommentSeamFacts {
    /// Build one seam fact snapshot.
    fn build(context: &CommentSeamContext<'_>) -> Self {
        let token_before_type = context.token_before_span.map(|token| token.token.ty);
        let token_after_type = context.token_after_span.map(|token| token.token.ty);
        let token_before_keyword =
            classify_comment_seam_keyword(context.file, context.token_before_span);
        let token_after_keyword =
            classify_comment_seam_keyword(context.file, context.token_after_span);
        let has_leading_newline = context.trivia.boundary.newlines.has_leading_newline();
        let has_trailing_newline = context.trivia.boundary.newlines.has_trailing_newline();
        let comment_style = context.tree.get(context.trivia.comment).style;
        let comment_is_line = comment_style == ast::CommentStyle::Slash;
        let comment_is_star = comment_style == ast::CommentStyle::Star;
        let comment_is_multiline_star =
            comment_is_star && context.file.span_str(context.trivia.span).contains('\n');
        let token_after_prefers_left =
            token_after_type.is_some_and(token_after_prefers_left_ownership);
        let token_before_is_control_head_close_paren = context.token_before.is_some_and(|index| {
            token_is_control_head_close_paren(context.file, context.semantic_tokens, index)
        });
        let token_before_is_return_type_colon = context
            .token_before
            .and_then(|index| previous_non_newline_token_index(context.semantic_tokens, index))
            .is_some_and(|index| {
                context.semantic_tokens[index].token.ty == TokenType::CloseParenthesis
            })
            && token_before_type == Some(TokenType::Colon);
        let seam_binds_right = token_before_type.is_some_and(is_open_delimiter_token)
            || matches!(
                token_before_type,
                Some(TokenType::Arrow | TokenType::ArrowWide)
            )
            || token_before_keyword == CommentSeamKeyword::Export
            || token_before_keyword == CommentSeamKeyword::Satisfies
            || token_before_keyword == CommentSeamKeyword::As
            || token_before_type == Some(TokenType::Assign);

        Self {
            has_leading_newline,
            has_trailing_newline,
            comment_is_line,
            comment_is_star,
            comment_is_multiline_star,
            token_before_type,
            token_after_type,
            token_before_keyword,
            token_after_keyword,
            token_after_prefers_left,
            token_before_is_control_head_close_paren,
            token_before_is_return_type_colon,
            seam_binds_right,
        }
    }

    /// Return whether token before seam has one type.
    #[inline]
    fn token_before_is(self, token_type: TokenType) -> bool {
        self.token_before_type == Some(token_type)
    }

    /// Return whether token after seam has one type.
    #[inline]
    fn token_after_is(self, token_type: TokenType) -> bool {
        self.token_after_type == Some(token_type)
    }

    /// Return whether token before seam is one keyword.
    #[inline]
    fn token_before_is_keyword(self, keyword: CommentSeamKeyword) -> bool {
        self.token_before_keyword == keyword
    }

    /// Return whether token after seam is one keyword.
    #[inline]
    fn token_after_is_keyword(self, keyword: CommentSeamKeyword) -> bool {
        self.token_after_keyword == keyword
    }

    /// Return whether token after seam starts one switch label.
    #[inline]
    fn token_after_is_case_or_default(self) -> bool {
        self.token_after_keyword == CommentSeamKeyword::Case
            || self.token_after_keyword == CommentSeamKeyword::Default
    }
}

/// Mutable caches for one seam rule evaluation.
#[derive(Default)]
struct CommentSeamRuleState {
    /// Lazily resolved smallest owner that encloses seam token range.
    seam_owner: Option<u32>,
    /// Whether seam owner lookup was executed.
    seam_owner_resolved: bool,
}

/// Resolve one seam owner lazily from seam token range.
fn resolve_comment_seam_owner(
    context: &CommentSeamContext<'_>,
    state: &mut CommentSeamRuleState,
) -> Option<u32> {
    if state.seam_owner_resolved {
        return state.seam_owner;
    }

    state.seam_owner_resolved = true;
    state.seam_owner = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(context.tree, before.span.start, after.span.end)
        });
    state.seam_owner
}

/// Resolve one comment trivia target owner and position from token seams.
/// FUGU #Cleanup: clean up the unholy formatter trivia attachment
fn resolve_formatter_comment_trivia_attachment(
    file: &File,
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    trivia: destack_ast::CommentTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
) -> (Option<u32>, AnnotationPosition) {
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

    // delimiter interiors use container infix placement
    if let (Some(token_before_span), Some(token_after_span)) = (token_before_span, token_after_span)
    {
        if is_open_delimiter_token(token_before_span.token.ty)
            && is_close_delimiter_token(token_after_span.token.ty)
            && delimiters_match(token_before_span.token.ty, token_after_span.token.ty)
            && let Some(container_owner) = find_smallest_owner_enclosing_range(
                tree,
                token_before_span.span.start,
                token_after_span.span.end,
            )
        {
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
            return (Some(target_node), AnnotationPosition::BlockInfix);
        }
    }

    // comments between parameter name and type belong to the whole parameter owner
    if token_after_span.is_some_and(|token| token.token.ty == TokenType::Colon)
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && let Some(owner) = lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
        && tree.get_node_type(owner) == NodeType::Parameter
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, owner);
        return (Some(target_node), AnnotationPosition::LinePrefix);
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
    let facts = CommentSeamFacts::build(&context);
    let mut state = CommentSeamRuleState::default();

    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_line = facts.comment_is_line;
    let comment_is_star = facts.comment_is_star;
    let comment_is_multiline_star = facts.comment_is_multiline_star;
    let token_after_is_at = facts.token_after_is(TokenType::At);
    let token_after_is_arrow = matches!(
        facts.token_after_type,
        Some(TokenType::Arrow | TokenType::ArrowWide)
    );
    let token_after_is_case_or_default = facts.token_after_is_case_or_default();
    let token_after_is_open_brace = facts.token_after_is(TokenType::OpenBrace);
    let token_after_is_open_parenthesis = facts.token_after_is(TokenType::OpenParenthesis);
    let token_after_is_colon = facts.token_after_is(TokenType::Colon);
    let token_after_is_close_parenthesis = facts.token_after_is(TokenType::CloseParenthesis);
    let token_after_is_open_bracket = facts.token_after_is(TokenType::OpenBracket);
    let token_after_is_dot = facts.token_after_is(TokenType::Dot);
    let token_after_is_maybe = facts.token_after_is(TokenType::Maybe);
    let token_after_is_semicolon = facts.token_after_is(TokenType::Semicolon);
    let token_after_is_less_than = facts.token_after_is(TokenType::LessThan);
    let token_after_is_as = facts.token_after_is_keyword(CommentSeamKeyword::As);
    let token_after_is_satisfies = facts.token_after_is_keyword(CommentSeamKeyword::Satisfies);
    let token_after_is_const = facts.token_after_is_keyword(CommentSeamKeyword::Const);
    let token_after_is_chain_or_index_boundary = token_after_is_dot || token_after_is_open_bracket;

    let token_before_is_export = facts.token_before_is_keyword(CommentSeamKeyword::Export);
    let token_before_is_satisfies = facts.token_before_is_keyword(CommentSeamKeyword::Satisfies);
    let token_before_is_as = facts.token_before_is_keyword(CommentSeamKeyword::As);
    let token_before_is_implements = facts.token_before_is_keyword(CommentSeamKeyword::Implements);
    let token_before_is_comma = facts.token_before_is(TokenType::Comma);
    let token_before_is_less_than = facts.token_before_is(TokenType::LessThan);
    let token_before_is_open_parenthesis = facts.token_before_is(TokenType::OpenParenthesis);
    let token_before_is_open_brace = facts.token_before_is(TokenType::OpenBrace);
    let token_before_is_semicolon = facts.token_before_is(TokenType::Semicolon);
    let token_before_is_assign = facts.token_before_is(TokenType::Assign);
    let token_before_is_spread = facts.token_before_is(TokenType::Spread);
    let token_before_is_control_head_close_paren = facts.token_before_is_control_head_close_paren;
    let token_before_is_return_type_colon = facts.token_before_is_return_type_colon;

    // same-line comments after no-semi guards should stay on the guarded expression
    if token_before_is_semicolon
        && token_after_is_open_parenthesis
        && comment_is_star
        && let Some(mut target_node) = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
    {
        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if !matches!(tree.get(expression_id), Expression::Parenthesized { .. }) {
                target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            }
        } else {
            target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        }

        let position = if has_leading_newline {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return (Some(target_node), position);
    }

    // comments between rest spread and binding names stay on the parameter owner
    if !has_leading_newline
        && !has_trailing_newline
        && token_before_is_spread
        && comment_is_star
        && let Some(target_node) = resolve_comment_seam_owner(&context, &mut state)
            .or(left_owner)
            .or(right_owner)
    {
        let target_node =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Parameter)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    // declaration generic head seams should stay on the declaration head
    if !has_leading_newline && has_trailing_newline && token_after_is_less_than {
        let declaration_target = token_after
            .and_then(|token_after_index| {
                find_next_declaration_owner_from_token(tree, owner_index, token_after_index)
            })
            .or_else(|| {
                right_owner.and_then(|owner| {
                    promote_owner_to_declaration_ancestor(tree, parents, owner).or_else(|| {
                        (tree.get_node_type(owner) == NodeType::Declaration).then_some(owner)
                    })
                })
            })
            .or_else(|| {
                left_owner.and_then(|owner| {
                    promote_owner_to_declaration_ancestor(tree, parents, owner).or_else(|| {
                        (tree.get_node_type(owner) == NodeType::Declaration).then_some(owner)
                    })
                })
            });

        if let Some(target_node) = declaration_target {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::LinePrefix);
        }
    }

    // seam comments before chain and index operators stay with the left segment
    if !has_leading_newline
        && token_after_is_chain_or_index_boundary
        && !token_before_is_open_brace
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let target_node = token_before_span.map_or(target_node, |token| {
            promote_owner_by_shared_end(tree, parents, target_node, token.span.end)
        });
        if has_trailing_newline {
            return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
        }
        return (Some(target_node), AnnotationPosition::LinePostfix);
    }

    // comments between object open braces and computed keys stay inside the object
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_after_is_colon
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
    }

    // comments between object open braces and computed keys stay inside the object
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_before_is_open_brace
        && token_after_is_open_bracket
        && let Some(target_node) = resolve_comment_seam_owner(&context, &mut state)
    {
        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if let Expression::ObjectExpression { properties, .. } = tree.get(expression_id)
                && let Some(first_property) = properties.first().copied()
            {
                return (Some(first_property.id), AnnotationPosition::LinePrefix);
            }
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    // comments before closure-cast object literals stay with the rhs cast target
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && token_after_is_open_brace
        && token_before_is_open_parenthesis
        && let Some(mut target_node) = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
            .or_else(|| resolve_comment_seam_owner(&context, &mut state))
    {
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

        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if !matches!(tree.get(expression_id), Expression::Parenthesized { .. }) {
                target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            }
        } else {
            target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        }

        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    // seam comments before `as` and `satisfies` stay with the asserted left expression
    if !has_leading_newline
        && !has_trailing_newline
        && comment_is_star
        && (token_after_is_as || token_after_is_satisfies)
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePostfix);
    }

    // optional call line comments should stay on the full optional expression
    if !has_leading_newline
        && has_trailing_newline
        && token_after_is_maybe
        && comment_is_line
        && let Some(target_node) = resolve_comment_seam_owner(&context, &mut state)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePostfix);
    }

    // optional call block comments should stay on the left call segment
    if !has_leading_newline
        && !has_trailing_newline
        && token_after_is_maybe
        && comment_is_star
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePostfix);
    }

    // comments after `as` should resolve to the cast expression seam
    if token_before_is_as
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && let Some(target_node) = resolve_comment_seam_owner(&context, &mut state)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
    }

    // line comments after `satisfies` only move to rhs prefixes for multi-argument type paths
    if token_before_is_satisfies && !has_leading_newline && has_trailing_newline && comment_is_line
    {
        if let Some(right_target) = right_owner {
            let mut prefix_target = None;

            if tree.get_node_type(right_target) == NodeType::Expression {
                let right_expression = LocalNodeId::<Expression>::new(right_target);
                if matches!(
                    tree.get(right_expression),
                    Expression::Path {
                        static_arguments: Some(static_arguments),
                        ..
                    } if static_arguments.len() > 1
                ) {
                    prefix_target = Some(right_target);
                }
            }

            if let Some(prefix_target) = prefix_target {
                let prefix_target = normalize_formatter_trivia_target_owner(tree, prefix_target);
                return (Some(prefix_target), AnnotationPosition::LinePrefix);
            }
        }

        if let Some(target_node) = resolve_comment_seam_owner(&context, &mut state).or(left_owner) {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
        }
    }

    // line comments after `<` in satisfies rhs type arguments stay with the rhs type
    if token_before_is_less_than
        && !has_leading_newline
        && has_trailing_newline
        && comment_is_line
        && resolve_comment_seam_owner(&context, &mut state)
            .or(left_owner)
            .and_then(|target_node| {
                promote_owner_to_satisfies_expression_ancestor(tree, parents, target_node)
            })
            .is_some()
    {
        if let Some(right_target) = right_owner {
            let right_target = normalize_formatter_trivia_target_owner(tree, right_target);
            return (Some(right_target), AnnotationPosition::LinePrefix);
        }
    }

    // multiline comments between `as` and `const` stay after the assertion
    if token_before_is_as
        && token_after_is_const
        && !has_leading_newline
        && comment_is_multiline_star
        && let Some(target_node) = resolve_comment_seam_owner(&context, &mut state)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
    }

    // own-line comments between statements and semicolon guards belong to the next statement
    if has_leading_newline
        && token_after_is_semicolon
        && let Some(mut target_node) = token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(right_owner)
    {
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

        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // own-line comments before switch case labels should attach to the first case expression
    if has_leading_newline
        && token_after_is_case_or_default
        && let Some(target_node) = resolve_comment_seam_owner(&context, &mut state).or(right_owner)
    {
        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if let Expression::Match { cases, .. } = tree.get(expression_id)
                && let Some(first_case) = cases.first().copied()
            {
                return (Some(first_case.id), AnnotationPosition::BlockPrefix);
            }
        }

        if tree.get_node_type(target_node) == NodeType::Block {
            let block_id = LocalNodeId::<Block>::new(target_node);
            let (target_node, position) = resolve_block_leading_comment_target(tree, block_id);
            return (Some(target_node), position);
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // own-line comments after `implements` should stay on the class declaration seam
    if has_leading_newline
        && token_before_is_implements
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // comments directly before decorators should bind to the right declaration owner
    if token_after_is_at {
        let declaration_target = token_after.and_then(|token_after_index| {
            find_next_declaration_owner_from_token(tree, owner_index, token_after_index)
        });
        let target_node = declaration_target.or_else(|| {
            right_owner
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
                .or(right_owner)
        });
        if let Some(target_node) = target_node {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            if has_leading_newline {
                return (Some(target_node), AnnotationPosition::BlockPrefix);
            }
            return (Some(target_node), AnnotationPosition::LinePrefix);
        }
    }

    // comments between parameter list and arrow belong to the enclosing arrow expression
    if token_after_is_arrow
        && let (Some(token_before_span), Some(token_after_span)) =
            (token_before_span, token_after_span)
        && let Some(owner) = find_smallest_owner_enclosing_range(
            tree,
            token_before_span.span.start,
            token_after_span.span.end,
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, owner);
        if has_leading_newline {
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }
        return (Some(target_node), AnnotationPosition::LinePrefix);
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
        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    // trailing line comments after control heads should stay before the body statement
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_control_head_close_paren
        && !token_after_is_case_or_default
        && comment_is_line
    {
        if let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
            && let Some(shared_owner) =
                lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
            && tree.get_node_type(shared_owner) == NodeType::Expression
        {
            let shared_expression = LocalNodeId::<Expression>::new(shared_owner);
            match tree.get(shared_expression) {
                Expression::If {
                    then_expression, ..
                } => {
                    if matches!(tree.get(*then_expression), Expression::Block(_)) {
                        let then_block_expression = *then_expression;
                        let block_id =
                            if let Expression::Block(block_id) = tree.get(then_block_expression) {
                                *block_id
                            } else {
                                unreachable!()
                            };
                        let (target_node, position) =
                            resolve_block_leading_comment_target(tree, block_id);
                        return (Some(target_node), position);
                    }
                    return (Some(then_expression.id), AnnotationPosition::BlockPrefix);
                }
                Expression::While { body, .. }
                | Expression::ForEach { body, .. }
                | Expression::For { body, .. }
                | Expression::Loop { body } => {
                    let (target_node, position) = resolve_block_leading_comment_target(tree, *body);
                    return (Some(target_node), position);
                }
                _ => {}
            }
        }

        if let Some(target_node) = right_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }
    }

    // return type seam comments should stay between `:` and the return type
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_return_type_colon
        && comment_is_line
        && let Some(target_node) = right_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    // parameter trailing comments before `)` should stay attached to the parameter
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_comma
        && token_after_is_close_parenthesis
        && comment_is_line
        && let Some(target_node) = left_owner
    {
        let target_node =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Parameter)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
    }

    // trailing comments after callback arguments should stay with the callback argument
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_comma
        && !token_after_is_close_parenthesis
        && comment_is_line
        && let Some(target_node) = left_owner
    {
        let target_node =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Argument)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
    }

    // comments between assignment and rhs should bind to the rhs seam
    if !has_leading_newline
        && has_trailing_newline
        && token_before_is_assign
        && let Some(mut target_node) =
            right_owner.or_else(|| resolve_comment_seam_owner(&context, &mut state))
    {
        target_node = token_after_span.map_or(target_node, |token| {
            promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
        });
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
        let comment_starts_on_assign_line = token_before_span.is_some_and(|before_token| {
            let before_line = file
                .get_position(before_token.span.start)
                .map_or(0, |position| position.0);
            let comment_line = file
                .get_position(trivia.span.start)
                .map_or(0, |position| position.0);
            before_line == comment_line
        });
        if comment_is_line && comment_starts_on_assign_line {
            return (Some(target_node), AnnotationPosition::LinePrefix);
        }
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // own-line comments between assignment and rhs stay on the rhs value region
    if has_leading_newline
        && token_before_is_assign
        && let Some(mut target_node) =
            right_owner.or_else(|| resolve_comment_seam_owner(&context, &mut state))
    {
        target_node = token_after_span.map_or(target_node, |token| {
            promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
        });
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

        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // comments between method signatures and opening braces should stay inside the body
    if !has_leading_newline
        && has_trailing_newline
        && token_after_is_open_brace
        && let Some(target_node) = right_owner
    {
        if tree.get_node_type(target_node) == NodeType::Block {
            let block_id = LocalNodeId::<Block>::new(target_node);
            let (target_node, position) = resolve_block_leading_comment_target(tree, block_id);
            return (Some(target_node), position);
        }

        if tree.get_node_type(target_node) == NodeType::Declaration {
            return (Some(target_node), AnnotationPosition::BlockInfix);
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    resolve_formatter_comment_trivia_fallback(&context, facts, &mut state, left_owner, right_owner)
}

/// Resolve fallback comment trivia rules after specialized seam cases.
fn resolve_formatter_comment_trivia_fallback(
    context: &CommentSeamContext<'_>,
    facts: CommentSeamFacts,
    state: &mut CommentSeamRuleState,
    left_owner: Option<u32>,
    right_owner: Option<u32>,
) -> (Option<u32>, AnnotationPosition) {
    let tree = context.tree;
    let parents = context.parents;
    let token_before_span = context.token_before_span;
    let token_before = context.token_before;
    let token_after = context.token_after;

    let has_leading_newline = facts.has_leading_newline;
    let has_trailing_newline = facts.has_trailing_newline;
    let comment_is_multiline_star = facts.comment_is_multiline_star;
    let token_after_is_else = facts.token_after_is_keyword(CommentSeamKeyword::Else);
    let token_before_is_open_delimiter =
        facts.token_before_type.is_some_and(is_open_delimiter_token);
    let token_after_is_less_than = facts.token_after_is(TokenType::LessThan);
    let token_after_prefers_left = facts.token_after_prefers_left;
    let seam_binds_right = facts.seam_binds_right;

    // own-line comments before `else` should stay between the previous branch and `else`
    if has_leading_newline
        && token_after_is_else
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let target_node = token_before_span.map_or(target_node, |token| {
            promote_owner_by_shared_end(tree, parents, target_node, token.span.end)
        });
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // own-line comments inside parenthesized groups before `<` should stay with the right side
    if has_leading_newline
        && token_before_is_open_delimiter
        && token_after_is_less_than
        && let Some(target_node) = resolve_comment_seam_owner(context, state).or(right_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // own-line comments before separators and closers belong to the left owner
    if has_leading_newline
        && token_after_prefers_left
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let target_node = token_before_span.map_or(target_node, |token| {
            promote_owner_by_shared_end(tree, parents, target_node, token.span.end)
        });
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // default own-line comment binding: right owner
    if has_leading_newline && let Some(target_node) = right_owner {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // inline multiline block comments before line-end should break to postfix blocks
    if comment_is_multiline_star
        && has_trailing_newline
        && !has_leading_newline
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let target_node = token_before_span.map_or(target_node, |token| {
            promote_owner_by_shared_end(tree, parents, target_node, token.span.end)
        });
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // default trailing-line comment binding: left owner
    if has_trailing_newline || token_after.is_none() {
        if seam_binds_right
            && !token_after_prefers_left
            && let Some(target_node) = right_owner
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::LinePrefix);
        }

        if let Some(target_node) = left_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            let target_node = token_before_span.map_or(target_node, |token| {
                promote_owner_by_shared_end(tree, parents, target_node, token.span.end)
            });
            if token_after_prefers_left {
                return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
            }
            return (Some(target_node), AnnotationPosition::LinePostfixBoundary);
        }
    }

    // same-line seams that precede separators and operators prefer left postfix
    if token_after_prefers_left && let Some(target_node) = left_owner {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let target_node = token_before_span.map_or(target_node, |token| {
            promote_owner_by_shared_end(tree, parents, target_node, token.span.end)
        });
        return (Some(target_node), AnnotationPosition::LinePostfix);
    }

    // same-line seams prefer the right owner as line-prefix
    if let Some(target_node) = right_owner {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    // fallback to left owner as line-postfix
    if let Some(target_node) = left_owner {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let target_node = token_before_span.map_or(target_node, |token| {
            promote_owner_by_shared_end(tree, parents, target_node, token.span.end)
        });
        return (Some(target_node), AnnotationPosition::LinePostfix);
    }

    // comment-only files can still anchor to one enclosing owner
    if token_before.is_none()
        && token_after.is_none()
        && let Some(target_node) = find_smallest_owner_enclosing_range(
            tree,
            context.trivia.span.start,
            context.trivia.span.end,
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    (None, AnnotationPosition::BlockInfix)
}

/// Resolve one blank trivia target owner and position from token seams.
fn resolve_formatter_blank_trivia_attachment(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    trivia: destack_ast::BlankTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    seam_index: &FormatterTriviaSeamIndex,
    parents: &NodeParentIndex,
) -> (Option<u32>, AnnotationPosition) {
    let token_before = decode_token_index(trivia.boundary.token_before);
    let token_after = decode_token_index(trivia.boundary.token_after);
    let token_before_span = token_before
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let token_after_span = token_after
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let seam_has_line_comment = seam_index.line_comment_seams.contains(&encode_trivia_seam(
        trivia.boundary.token_before,
        trivia.boundary.token_after,
    ));

    let right_owner = token_after
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
    let left_owner = token_before
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

    let token_after_is_at = token_after_span.is_some_and(|token| token.token.ty == TokenType::At);
    let token_after_is_semicolon =
        token_after_span.is_some_and(|token| token.token.ty == TokenType::Semicolon);
    let token_after_is_close_brace =
        token_after_span.is_some_and(|token| token.token.ty == TokenType::CloseBrace);
    let token_after_is_open_parenthesis =
        token_after_span.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis);
    let token_before_is_open_parenthesis =
        token_before_span.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis);
    let token_before_is_statement_end = token_before_span.is_some_and(|token| {
        matches!(
            token.token.ty,
            TokenType::Semicolon | TokenType::CloseBrace | TokenType::CloseParenthesis
        )
    });
    let token_after_starts_statement = token_after_span.is_some_and(|token| {
        matches!(
            token.token.ty,
            TokenType::Identifier | TokenType::At | TokenType::OpenParenthesis
        )
    });
    let right_owner_is_argument =
        right_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Argument);

    // blank seams that already contain line comments should not add extra spacing
    if seam_has_line_comment
        && !token_before_span.is_some_and(|token| token.token.ty == TokenType::Assign)
    {
        return (None, AnnotationPosition::BlockInfix);
    }

    // blank seams before semicolons or closing braces are formatting noise
    if token_after_is_semicolon || token_after_is_close_brace {
        return (None, AnnotationPosition::BlockInfix);
    }

    // blanks before decorators should stay before the decorated declaration
    if token_after_is_at {
        if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
            && let Some(token_after_index) = token_after
            && let Some(target_node) =
                find_next_member_owner_from_token(tree, owner_index, token_after_index)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
            && let Some(target_node) = left_owner
            && tree.get_node_type(target_node) == NodeType::Member
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPostfix);
        }

        if let Some(token) = token_after_span
            && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token.span)
            && tree.get_node_type(target_node) == NodeType::Member
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        if let Some(target_node) = right_owner
            && tree.get_node_type(target_node) == NodeType::Member
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        let declaration_target = token_after
            .and_then(|token_after_index| {
                find_next_declaration_owner_from_token(tree, owner_index, token_after_index)
            })
            .or_else(|| {
                right_owner
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
                    .or(right_owner)
            });
        if let Some(target_node) = declaration_target {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }
    }

    // top-level expression seams already carry spacing in statement-list formatting
    if token_before_is_statement_end
        && token_after_starts_statement
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Expression
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_none()
    {
        return (None, AnnotationPosition::BlockInfix);
    }

    // top-level expression-to-declaration seams already carry spacing in statement-list formatting
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Expression
        && tree.get_node_type(right_owner) == NodeType::Declaration
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_none()
    {
        return (None, AnnotationPosition::BlockInfix);
    }

    // top-level declaration-to-expression seams after close braces don't need blank trivia
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::CloseBrace)
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Declaration
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_none()
    {
        return (None, AnnotationPosition::BlockInfix);
    }

    // block-local seams before await expressions don't need extra blank trivia
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Expression
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_some()
    {
        let right_expression = LocalNodeId::<Expression>::new(right_owner);
        if matches!(tree.get(right_expression), Expression::Await { .. }) {
            return (None, AnnotationPosition::BlockInfix);
        }
    }

    // semicolon seams before block-local member-path expressions don't carry independent blank trivia
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
        && let Some(right_owner) = right_owner
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_some()
    {
        let right_expression = LocalNodeId::<Expression>::new(right_owner);
        if let Expression::Path { path, .. } = tree.get(right_expression)
            && path.segments.len() > 1
        {
            return (None, AnnotationPosition::BlockInfix);
        }
    }

    // top-level seams after declaration close braces and before plain expressions
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::CloseBrace)
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_none()
        && promote_owner_to_declaration_ancestor(tree, parents, left_owner).is_some()
    {
        let right_expression = LocalNodeId::<Expression>::new(right_owner);
        if !matches!(tree.get(right_expression), Expression::Export { .. }) {
            return (None, AnnotationPosition::BlockInfix);
        }
    }

    // assignment seams should keep blank separators with the rhs value owner
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::Assign)
        && token_after_span.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis)
        && let Some(mut target_node) = right_owner
    {
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

        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // blank seams between callees and argument lists are formatting noise
    if right_owner_is_argument && token_after_is_open_parenthesis {
        return (None, AnnotationPosition::BlockInfix);
    }

    // blank seams right after `(` before first arguments are formatting noise
    if right_owner_is_argument && token_before_is_open_parenthesis {
        return (None, AnnotationPosition::BlockInfix);
    }

    if let Some(target_node) = right_owner {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    if let Some(target_node) = left_owner {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    (None, AnnotationPosition::BlockInfix)
}

fn build_formatter_annotation_projection(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    _side_tokens: &[TokenSpan],
    _side_span: &MultiSpan,
    parents: &NodeParentIndex,
) -> (
    Vec<FormatterAnnotationEntry>,
    Vec<SmallVec<[LocalNodeId<Annotation>; 4]>>,
) {
    let node_count = tree.next_id() as usize;
    let mut entries = Vec::new();
    let mut by_node_id = vec![SmallVec::new(); node_count];

    // add parser semantic annotations first
    for (&target_id, annotation_ids) in tree.get_all_annotations() {
        if target_id as usize >= by_node_id.len() {
            continue;
        }

        for &annotation_id in annotation_ids {
            let ast_annotation = tree.get(annotation_id);
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
            by_node_id[target_id as usize].push(local_id);
        }
    }

    // build formatter-side owner indexes for trivia placement
    let owner_index = build_formatter_trivia_owner_index(tree, tokens);
    let seam_index = build_formatter_trivia_seam_index(tree);

    // add comment trivia with formatter-side placement resolution
    for trivia in tree.comment_trivia().iter().copied() {
        let (target_id, position) = resolve_formatter_comment_trivia_attachment(
            file,
            tree,
            tokens,
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
        let (target_id, position) = resolve_formatter_blank_trivia_attachment(
            tree,
            tokens,
            trivia,
            &owner_index,
            &seam_index,
            parents,
        );

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

/// Cached argument annotation facts used by hot call formatting paths.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachedArgumentAnnotationProfile {
    /// Whether the argument has any comment annotation.
    pub has_comment: bool,
    /// Whether the argument has a trailing slash style comment annotation.
    pub has_line_comment: bool,
    /// Whether the argument has a slash style prefix comment annotation.
    pub has_prefix_line_comment: bool,
    /// Whether the argument has any non-blank prefix annotation.
    pub has_prefix_annotation: bool,
}

/// Cached regular call argument expansion profile keyed by call expression node id.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachedCallArgumentExpansionProfile {
    /// The final force expand decision.
    pub force_expand: bool,
    /// Whether the call has a non blank infix annotation.
    pub has_call_infix_annotations: bool,
    /// Whether the last argument is a collection literal.
    pub trailing_collection_argument: bool,
}

/// Cached regular and chain call argument expansion profiles keyed by call expression node id.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachedCallArgumentExpansionProfiles {
    /// Cached regular call expansion profile.
    pub regular: CachedCallArgumentExpansionProfile,
    /// Cached chain call force-expand decision.
    pub chain_force_expand: bool,
}

/// Cached call argument layout-class facts keyed by call expression node id.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachedCallArgumentLayoutClass {
    /// Whether the call has a non-blank infix annotation.
    pub has_call_infix_annotations: bool,
    /// Whether any dynamic argument has annotations.
    pub has_any_argument_annotation: bool,
    /// Whether argument source between first and last spans multiple lines.
    pub is_multiline_in_source: bool,
    /// Whether all dynamic arguments are single-line and unannotated.
    pub all_single_line_and_unannotated: bool,
    /// Whether all dynamic arguments are compact, simple, and unannotated.
    pub all_compact_simple_unannotated: bool,
    /// Whether all arguments can use the plain argument writer.
    pub all_plain_call_arguments: bool,
    /// Whether any argument has a line comment annotation.
    pub has_line_comment_annotations: bool,
    /// Whether any argument is a block callback.
    pub has_block_callback_argument: bool,
    /// Whether the first argument is a block callback.
    pub first_argument_is_block_callback: bool,
    /// Whether the last argument is a block callback.
    pub last_argument_is_block_callback: bool,
    /// Whether any non-last non-callback argument is non-trivial.
    pub has_non_trivial_non_callback_argument: bool,
    /// Number of block callback arguments before the last argument.
    pub non_last_block_callback_count: usize,
    /// Index of the first block callback before the last argument.
    pub non_last_block_callback_index: Option<usize>,
    /// Number of lambda arguments.
    pub arrow_argument_count: usize,
    /// Number of function expression arguments.
    pub function_argument_count: usize,
    /// Whether any argument is a spread argument.
    pub has_spread_argument: bool,
    /// Whether any non-callback argument is complex and non-tree.
    pub has_complex_non_callback_argument: bool,
    /// Whether this call has a parent postfix call-chain operation.
    pub has_call_chain_parent: bool,
    /// Whether the last dynamic argument is a collection literal.
    pub trailing_collection_argument: bool,
    /// Whether this call can force hug-last inline layout.
    pub force_hug_last_inline: bool,
}

/// Dense formatter node caches keyed by node id.
#[derive(Debug, Clone)]
struct FormatterNodeCaches {
    /// Cached span char lengths for node ids.
    node_span_char_len: Vec<Cell<u32>>,
    /// Cached node span newline predicates keyed by node id.
    node_has_newline: Vec<Cell<u8>>,
    /// Cached call argument expansion profiles for regular and chain modes keyed by call node id.
    call_argument_expansion_profiles: Vec<Cell<Option<CachedCallArgumentExpansionProfiles>>>,
    /// Cached inline call length estimates without static arguments keyed by call expression id.
    call_inline_len_without_static_arguments: Vec<Cell<Option<Option<usize>>>>,
    /// Cached call argument annotation profiles keyed by argument node id.
    argument_annotation_profile: Vec<Cell<Option<CachedArgumentAnnotationProfile>>>,
    /// Cached compact simple unannotated argument predicate keyed by argument node id.
    argument_compact_simple_unannotated: Vec<Cell<Option<bool>>>,
    /// Cached plain-call-argument predicate keyed by argument node id.
    argument_plain_call_argument: Vec<Cell<Option<bool>>>,
    /// Cached call argument layout-class facts keyed by call expression node id.
    call_argument_layout_class: Vec<Cell<Option<CachedCallArgumentLayoutClass>>>,
    /// Cached boundary-comment presence keyed by call expression node id.
    call_argument_boundary_comments: Vec<Cell<Option<bool>>>,
    /// Cached chain call force-expand decisions keyed by call expression node id.
    call_argument_chain_force_expand: Vec<Cell<Option<bool>>>,
    /// Cached transparent inner expression ids keyed by expression node id.
    transparent_inner_expression: Vec<Cell<Option<LocalNodeId<Expression>>>>,
    /// Cached type-context decisions keyed by expression node id.
    expression_type_context: Vec<Cell<u8>>,
    /// Cached template interpolation ancestry decisions keyed by expression node id.
    expression_template_interpolation: Vec<Cell<u8>>,
    /// Cached type-conditional ancestry decisions keyed by expression node id.
    expression_type_conditional_ancestor: Vec<Cell<u8>>,
}

impl FormatterNodeCaches {
    /// Build all dense formatter node caches.
    fn new(node_count: usize) -> Self {
        Self {
            node_span_char_len: vec![Cell::new(NODE_SPAN_CHAR_LEN_UNKNOWN); node_count],
            node_has_newline: vec![Cell::new(NODE_BOOL_STATE_UNKNOWN); node_count],
            call_argument_expansion_profiles: vec![Cell::new(None); node_count],
            call_inline_len_without_static_arguments: vec![Cell::new(None); node_count],
            argument_annotation_profile: vec![Cell::new(None); node_count],
            argument_compact_simple_unannotated: vec![Cell::new(None); node_count],
            argument_plain_call_argument: vec![Cell::new(None); node_count],
            call_argument_layout_class: vec![Cell::new(None); node_count],
            call_argument_boundary_comments: vec![Cell::new(None); node_count],
            call_argument_chain_force_expand: vec![Cell::new(None); node_count],
            transparent_inner_expression: vec![Cell::new(None); node_count],
            expression_type_context: vec![Cell::new(TYPE_CONTEXT_STATE_UNKNOWN); node_count],
            expression_template_interpolation: vec![
                Cell::new(TYPE_CONTEXT_STATE_UNKNOWN);
                node_count
            ],
            expression_type_conditional_ancestor: vec![
                Cell::new(TYPE_CONTEXT_STATE_UNKNOWN);
                node_count
            ],
        }
    }
}

/// Destack format options.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DestackFormatOptions {
    // source
    /// The source language type.
    pub language_type: LanguageType = LanguageType::Destack,

    // layout
    /// The type of line ending to apply to the printed input.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u16 = 100,

    // syntax
    /// Quote style for string literals.
    pub quote_style: QuoteStyle = QuoteStyle::Semantic,
    /// Trailing comma policy for multi-line constructs.
    pub trailing_comma: TrailingComma = TrailingComma::All,
    /// Spaces inside object braces: `{ foo }` (true) vs `{foo}` (false).
    pub bracket_spacing: bool = true,
    /// Arrow function parentheses policy.
    pub arrow_parentheses: ArrowParentheses = ArrowParentheses::Always,
    /// Object property quoting policy.
    pub quote_props: QuoteProperty = QuoteProperty::AsNeeded,

    // tree/jsx
    /// Put `>` of multi-line tree/JSX on same line as last attribute.
    pub bracket_same_line: bool = false,
    /// Force each tree/JSX attribute onto its own line.
    pub single_attribute_per_line: bool = false,

    // imports
    /// Whether to organize/sort imports and exports.
    pub organize_imports: OrganizeImports = OrganizeImports::Off,
    /// Sort order for import/export specifiers within `{ }`.
    pub import_sort_order: ImportSortOrder = ImportSortOrder::Natural,
    /// Respect file-level formatter ignore directives.
    pub respect_file_ignore: bool = true,
}

impl DestackFormatOptions {
    /// Default options with a given line width.
    pub fn default_with_line_width(line_width: u16) -> Self {
        Self {
            line_width,
            ..Self::default()
        }
    }

    /// Default options with tab indent style.
    pub fn default_tab() -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            ..Self::default()
        }
    }

    /// Default options with tab indent style and a given line width.
    pub fn default_tab_with_line_width(line_width: u16) -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            line_width,
            ..Self::default()
        }
    }

    /// Set the line ending.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Set the indent style.
    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.indent_style = indent_style;
        self
    }

    /// Set the indent width.
    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u16) -> Self {
        self.line_width = line_width;
        self
    }

    /// Set whether file-level formatter ignore directives are respected.
    pub fn with_respect_file_ignore(mut self, respect_file_ignore: bool) -> Self {
        self.respect_file_ignore = respect_file_ignore;
        self
    }

    /// Convert to print options (clamps line_width to u8 max).
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width.min(255) as u8,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }

    /// Create from workspace FormatterOptions.
    pub fn from_formatter_options(options: FormatterOptions, language_type: LanguageType) -> Self {
        Self {
            language_type,
            line_ending: options.line_ending,
            indent_style: options.indent_style,
            indent_width: options.indent_width,
            line_width: options.line_width,
            quote_style: options.quote_style,
            trailing_comma: options.trailing_comma,
            bracket_spacing: options.bracket_spacing,
            arrow_parentheses: options.arrow_parentheses,
            quote_props: options.quote_property,
            bracket_same_line: options.bracket_same_line,
            single_attribute_per_line: options.single_attribute_per_line,
            organize_imports: options.organize_imports,
            import_sort_order: options.import_sort_order,
            respect_file_ignore: true,
        }
    }
}

impl From<FormatterOptions> for DestackFormatOptions {
    fn from(options: FormatterOptions) -> Self {
        Self::from_formatter_options(options, LanguageType::Destack)
    }
}

impl FormatOptions for DestackFormatOptions {
    #[inline]
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    #[inline]
    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    #[inline]
    fn line_width(&self) -> u8 {
        self.line_width.min(255) as u8
    }

    #[inline]
    fn as_print_options(&self) -> PrintOptions {
        self.as_print_options()
    }
}

/// Destack format context.
#[derive(Debug, Clone)]
pub struct DestackFormatContext<'a> {
    /// The format options.
    pub options: DestackFormatOptions,
    /// The file.
    pub file: &'a File,
    /// The main tokens.
    pub tokens: &'a Vec<TokenSpan>,
    /// The side tokens.
    pub side_tokens: &'a Vec<TokenSpan>,
    /// The side span.
    pub side_span: &'a MultiSpan,
    /// The tree.
    pub tree: &'a NodeTree,
    /// The source map.
    pub source_map: &'a NodeSourceMap,
    /// The parent index.
    pub parents: NodeParentIndex,
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
    /// The current argument list group id, if any.
    pub current_argument_group_id: Option<GroupId>,
    /// Formatter-owned annotation entries (semantic + placed trivia).
    pub formatter_annotation_entries: Vec<FormatterAnnotationEntry>,
    /// Formatter-owned annotation ids grouped by target node id.
    pub formatter_annotation_ids_by_node_id: Vec<SmallVec<[LocalNodeId<Annotation>; 4]>>,
    /// Cached node-to-annotation ids and annotation metadata for hot annotation lookups.
    pub annotation_ids_cache: RefCell<Vec<Option<CachedAnnotationData>>>,
    /// Cached annotation presence for node ids.
    pub annotation_presence_cache: Vec<Cell<u8>>,
    /// Cached source slices for repeated span lookups.
    pub span_text_cache: RefCell<FxHashMap<Span, &'a str>>,
    /// Cached char lengths for repeated span width checks.
    pub span_char_len_cache: RefCell<FxHashMap<Span, usize>>,
    /// Whether the file text is fully ASCII.
    pub source_is_ascii: bool,
    /// Cached newline byte offsets in file text.
    pub newline_offsets: OnceCell<Vec<u32>>,
    /// Cached newline checks for repeated span newline predicates.
    pub span_has_newline_cache: RefCell<FxHashMap<Span, bool>>,
    /// Cached comment checks for repeated span comment predicates.
    pub span_has_comment_cache: RefCell<FxHashMap<Span, bool>>,
    /// Dense formatter caches keyed by node id.
    node_caches: FormatterNodeCaches,
    /// Cached sorted comment tokens for ignore-range and comment-boundary scans.
    pub comment_tokens_cache: OnceCell<Vec<TokenSpan>>,
    /// Comment spans for this file, sorted by start position.
    pub comment_spans: Vec<Span>,
    /// Optional formatter timing collector.
    pub timings: Option<Rc<FormatterTimings>>,
    /// Whether file text contains formatter ignore directive markers.
    pub has_ignore_directive_markers: bool,
    /// Whether file text contains template literal markers.
    pub has_template_literal_markers: bool,
    /// Whether instrumentation counters should be collected.
    pub instrumentation_enabled: bool,
    /// Whether file-level ignore was applied during formatting.
    pub file_ignore_applied: Rc<Cell<bool>>,
    /// Shared cache instrumentation counters.
    pub cache_stats: Rc<FormatterCacheStatsCollector>,
    /// Shared generic instrumentation counters.
    pub counters: Rc<FormatterCountersCollector>,
}

/// Parser artifacts required to build a formatter context.
#[derive(Debug)]
pub struct DestackFormatArtifacts<'a> {
    /// The source file being formatted.
    pub file: &'a File,
    /// The parsed node tree for the source file.
    pub tree: &'a NodeTree,
    /// Primary parser tokens for the source file.
    pub tokens: &'a Vec<TokenSpan>,
    /// Side token stream for comments and other non-primary trivia.
    pub side_tokens: &'a Vec<TokenSpan>,
    /// Span map for side tokens.
    pub side_span: &'a MultiSpan,
    /// Shared string pool for interned string data.
    pub strings: &'a ImmutableStringPool,
    /// Precomputed parent index for fast ancestry lookups.
    pub parents: NodeParentIndex,
}

impl<'a> DestackFormatContext<'a> {
    /// Construct a formatting context from parse artifacts.
    pub fn new(options: DestackFormatOptions, artifacts: DestackFormatArtifacts<'a>) -> Self {
        Self::new_with_timings(options, artifacts, false)
    }

    /// Construct a formatting context from parse artifacts with optional timing collection.
    pub fn new_with_timings(
        options: DestackFormatOptions,
        artifacts: DestackFormatArtifacts<'a>,
        timings_enabled: bool,
    ) -> Self {
        let DestackFormatArtifacts {
            file,
            tree,
            tokens,
            side_tokens,
            side_span,
            strings,
            parents,
        } = artifacts;
        let (formatter_annotation_entries, formatter_annotation_ids_by_node_id) =
            build_formatter_annotation_projection(
                file,
                tree,
                tokens,
                side_tokens,
                side_span,
                &parents,
            );
        let node_count = tree.next_id() as usize;
        let node_caches = FormatterNodeCaches::new(node_count);
        let timings_enabled = timings_enabled || timings_enabled_from_env();
        let file_text = file.text();
        let has_ignore_directive_markers = file_text.contains("format-ignore")
            || file_text.contains("fmt-ignore")
            || file_text.contains("deno-fmt-ignore")
            || file_text.contains("prettier-ignore")
            || file_text.contains("biome-ignore format")
            || file_text.contains("oxfmt-ignore");
        let source_is_ascii = file_text.is_ascii();
        let has_template_literal_markers = file_text.contains('`');
        let mut comment_spans = tokens
            .iter()
            .chain(side_tokens.iter())
            .filter_map(|token| {
                matches!(
                    token.token.ty,
                    TokenType::LineComment
                        | TokenType::BlockComment
                        | TokenType::DocLineComment
                        | TokenType::DocBlockComment
                )
                .then_some(token.span)
            })
            .collect::<Vec<_>>();
        comment_spans.sort_by_key(|span| span.start);

        let annotation_presence_cache = vec![Cell::new(ANNOTATION_STATE_NONE); node_count];
        for (node_id, annotation_ids) in formatter_annotation_ids_by_node_id.iter().enumerate() {
            if annotation_ids.is_empty() {
                continue;
            }

            if let Some(annotation_state) = annotation_presence_cache.get(node_id) {
                annotation_state.set(ANNOTATION_STATE_PRESENT);
            }
        }

        Self {
            options,
            file,
            tokens,
            side_tokens,
            side_span,
            tree,
            source_map: &tree.source_map,
            parents,
            strings,
            current_argument_group_id: None,
            formatter_annotation_entries,
            formatter_annotation_ids_by_node_id,
            annotation_ids_cache: RefCell::new(vec![None; node_count]),
            annotation_presence_cache,
            span_text_cache: RefCell::new(FxHashMap::default()),
            span_char_len_cache: RefCell::new(FxHashMap::default()),
            source_is_ascii,
            newline_offsets: OnceCell::new(),
            span_has_newline_cache: RefCell::new(FxHashMap::default()),
            span_has_comment_cache: RefCell::new(FxHashMap::default()),
            node_caches,
            comment_tokens_cache: OnceCell::new(),
            comment_spans,
            timings: timings_enabled.then(|| Rc::new(FormatterTimings::default())),
            has_ignore_directive_markers,
            has_template_literal_markers,
            instrumentation_enabled: timings_enabled,
            file_ignore_applied: Rc::new(Cell::new(false)),
            cache_stats: Rc::new(FormatterCacheStatsCollector::default()),
            counters: Rc::new(FormatterCountersCollector::default()),
        }
    }

    /// Return whether this file may contain formatter ignore directives.
    #[inline]
    pub fn has_ignore_directive_markers(&self) -> bool {
        self.has_ignore_directive_markers
    }

    /// Return whether this file may contain template literals.
    #[inline]
    pub fn has_template_literal_markers(&self) -> bool {
        self.has_template_literal_markers
    }

    /// Mark that file-level ignore was applied.
    #[inline]
    pub fn mark_file_ignore_applied(&self) {
        self.file_ignore_applied.set(true);
    }

    /// Return whether file-level ignore was applied.
    #[inline]
    pub fn file_ignore_applied(&self) -> bool {
        self.file_ignore_applied.get()
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'a str {
        if !self.instrumentation_enabled {
            return self.file.span_str(span);
        }

        {
            let cache = self.span_text_cache.borrow();
            if let Some(span_str) = cache.get(&span) {
                if self.instrumentation_enabled {
                    self.cache_stats
                        .span_text_hits
                        .set(self.cache_stats.span_text_hits.get() + 1);
                }
                return span_str;
            }
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .span_text_misses
                .set(self.cache_stats.span_text_misses.get() + 1);
        }
        let span_str = self.file.span_str(span);
        self.span_text_cache.borrow_mut().insert(span, span_str);
        span_str
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &'a str {
        self.get_span_str(token.span)
    }

    /// Get one normalized comment payload string.
    #[inline]
    pub fn get_comment_text(&self, comment_id: LocalNodeId<Comment>) -> Cow<'a, str> {
        let comment_source = self.get_span_str(self.get_span(comment_id));
        normalize_comment_payload(comment_source)
    }

    /// Get comment tokens sorted by source position.
    #[inline]
    pub fn comment_tokens(&self) -> &[TokenSpan] {
        self.comment_tokens_cache.get_or_init(|| {
            let mut tokens: Vec<TokenSpan> = self
                .tokens
                .iter()
                .copied()
                .chain(self.side_tokens.iter().copied())
                .filter(|token| {
                    matches!(
                        token.token.ty,
                        TokenType::LineComment
                            | TokenType::BlockComment
                            | TokenType::DocLineComment
                            | TokenType::DocBlockComment
                    )
                })
                .collect();
            tokens.sort_by_key(|token| token.span.start);
            tokens
        })
    }

    /// Get the Unicode scalar count for a source span.
    #[inline]
    pub fn span_char_len(&self, span: Span) -> usize {
        {
            let cache = self.span_char_len_cache.borrow();
            if let Some(len) = cache.get(&span) {
                return *len;
            }
        }

        let len = if self.source_is_ascii {
            span.len() as usize
        } else {
            self.get_span_str(span).chars().count()
        };
        self.span_char_len_cache.borrow_mut().insert(span, len);
        len
    }

    /// Get the Unicode scalar count for one node span.
    #[inline]
    pub fn node_span_char_len<T>(&self, node_id: LocalNodeId<T>) -> usize
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let cached = self.node_caches.node_span_char_len[node_index].get();
        if cached != NODE_SPAN_CHAR_LEN_UNKNOWN {
            return cached as usize;
        }

        let len = self.span_char_len(self.get_span(node_id));
        #[expect(clippy::cast_possible_truncation)]
        self.node_caches.node_span_char_len[node_index].set(len as u32);

        len
    }

    /// Return whether one node span contains a newline.
    #[inline]
    pub fn node_has_newline<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let cached = self.node_caches.node_has_newline[node_index].get();
        if cached == NODE_BOOL_STATE_TRUE {
            return true;
        }
        if cached == NODE_BOOL_STATE_FALSE {
            return false;
        }

        let has_newline = self.has_newline(self.get_span(node_id));
        self.node_caches.node_has_newline[node_index].set(if has_newline {
            NODE_BOOL_STATE_TRUE
        } else {
            NODE_BOOL_STATE_FALSE
        });

        has_newline
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn get_node<T>(&self, node_id: LocalNodeId<T>) -> &T
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get(node_id)
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn get_node_type<T>(&self, node_id: LocalNodeId<T>) -> NodeType
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get_node_type(node_id.id)
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn get_parent<T>(&self, node_id: LocalNodeId<T>) -> Option<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let parent_id = self.parents.get(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_node_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn get_parent_by_id(&self, node_id: u32) -> Option<(u32, NodeType)> {
        let parent_id = self.parents.get_by_id(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_node_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get all ancestors of a node.
    #[inline]
    pub fn get_ancestors<T>(&self, node_id: LocalNodeId<T>) -> Vec<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.parents
            .get_ancestors(node_id)
            .into_iter()
            .map(|parent_id| {
                let parent_type = self.tree.get_node_type(parent_id);
                (parent_id, parent_type)
            })
            .collect()
    }

    /// Return whether any ancestor of a node matches the predicate.
    #[inline]
    pub fn any_ancestor<T, F>(&self, node_id: LocalNodeId<T>, mut predicate: F) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnMut(u32, NodeType) -> bool,
    {
        let mut current_id = node_id.id;
        while let Some(parent_id) = self.parents.get_by_id(current_id) {
            let parent_type = self.tree.get_node_type(parent_id);
            if predicate(parent_id, parent_type) {
                return true;
            }
            current_id = parent_id;
        }

        false
    }

    /// Return the transparent inner expression for one expression node.
    #[inline]
    pub fn transparent_inner_expression(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let node_index = node_id.id as usize;

        if let Some(inner_expression_id) = self
            .node_caches
            .transparent_inner_expression
            .get(node_index)
            .and_then(Cell::get)
        {
            return inner_expression_id;
        }

        let mut current_id = node_id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        loop {
            let current_index = current_id.id as usize;
            visited_expression_indices.push(current_index);

            if self.has_annotation(current_id) {
                break;
            }

            let next_id = match self.tree.get(current_id) {
                Expression::Await { expression }
                | Expression::AwaitMaybe { expression }
                | Expression::Parenthesized { expression } => Some(*expression),
                _ => None,
            };

            let Some(next_id) = next_id else {
                break;
            };
            current_id = next_id;
        }

        for expression_index in visited_expression_indices {
            if let Some(cache_state) = self
                .node_caches
                .transparent_inner_expression
                .get(expression_index)
            {
                cache_state.set(Some(current_id));
            }
        }

        current_id
    }

    /// Return a cached type-context value for one expression node.
    #[inline]
    pub fn cached_expression_type_context(&self, node_id: LocalNodeId<Expression>) -> Option<bool> {
        let node_index = node_id.id as usize;
        let state = self
            .node_caches
            .expression_type_context
            .get(node_index)
            .map(Cell::get)
            .unwrap_or(TYPE_CONTEXT_STATE_UNKNOWN);

        if state == TYPE_CONTEXT_STATE_TRUE {
            return Some(true);
        }
        if state == TYPE_CONTEXT_STATE_FALSE {
            return Some(false);
        }

        None
    }

    /// Cache one type-context value for one expression node.
    #[inline]
    pub fn set_cached_expression_type_context(
        &self,
        node_id: LocalNodeId<Expression>,
        is_type_context: bool,
    ) {
        let node_index = node_id.id as usize;
        let state = if is_type_context {
            TYPE_CONTEXT_STATE_TRUE
        } else {
            TYPE_CONTEXT_STATE_FALSE
        };
        if let Some(cache_state) = self.node_caches.expression_type_context.get(node_index) {
            cache_state.set(state);
        }
    }

    /// Return whether one expression appears in template-literal interpolation.
    #[inline]
    pub fn expression_is_in_template_literal_interpolation(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> bool {
        if !self.has_template_literal_markers() {
            return false;
        }

        let mut current_id = node_id.id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        let has_template_interpolation_ancestor = loop {
            let current_index = current_id as usize;
            let cached_state = self
                .node_caches
                .expression_template_interpolation
                .get(current_index)
                .map(Cell::get)
                .unwrap_or(TYPE_CONTEXT_STATE_UNKNOWN);
            if cached_state == TYPE_CONTEXT_STATE_TRUE {
                break true;
            }
            if cached_state == TYPE_CONTEXT_STATE_FALSE {
                break false;
            }

            visited_expression_indices.push(current_index);

            let Some(parent_id) = self.parents.get_by_id(current_id) else {
                break false;
            };

            let is_template_parent = self.tree.get_node_type(parent_id) == NodeType::Expression
                && matches!(
                    self.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::TemplateExpression { .. } | Expression::TypeTemplateLiteral { .. }
                );
            if is_template_parent {
                break true;
            }

            current_id = parent_id;
        };

        let cached_state = if has_template_interpolation_ancestor {
            TYPE_CONTEXT_STATE_TRUE
        } else {
            TYPE_CONTEXT_STATE_FALSE
        };
        for expression_index in visited_expression_indices {
            if let Some(cache_state) = self
                .node_caches
                .expression_template_interpolation
                .get(expression_index)
            {
                cache_state.set(cached_state);
            }
        }

        has_template_interpolation_ancestor
    }

    /// Return whether one expression has a type-conditional ancestor.
    #[inline]
    pub fn expression_has_type_conditional_ancestor(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut current_id = node_id.id;
        let mut visited_expression_indices: SmallVec<[usize; 8]> = SmallVec::new();

        let has_type_conditional_ancestor = loop {
            let current_index = current_id as usize;
            let cached_state = self
                .node_caches
                .expression_type_conditional_ancestor
                .get(current_index)
                .map(Cell::get)
                .unwrap_or(TYPE_CONTEXT_STATE_UNKNOWN);
            if cached_state == TYPE_CONTEXT_STATE_TRUE {
                break true;
            }
            if cached_state == TYPE_CONTEXT_STATE_FALSE {
                break false;
            }

            visited_expression_indices.push(current_index);

            let Some(parent_id) = self.parents.get_by_id(current_id) else {
                break false;
            };

            let is_type_conditional_parent = self.tree.get_node_type(parent_id)
                == NodeType::Expression
                && matches!(
                    self.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::TypeConditional { .. }
                );
            if is_type_conditional_parent {
                break true;
            }

            current_id = parent_id;
        };

        let cached_state = if has_type_conditional_ancestor {
            TYPE_CONTEXT_STATE_TRUE
        } else {
            TYPE_CONTEXT_STATE_FALSE
        };
        for expression_index in visited_expression_indices {
            if let Some(cache_state) = self
                .node_caches
                .expression_type_conditional_ancestor
                .get(expression_index)
            {
                cache_state.set(cached_state);
            }
        }

        has_type_conditional_ancestor
    }

    /// Return the first ancestor of a node that matches the predicate.
    #[inline]
    pub fn find_ancestor<T, F>(
        &self,
        node_id: LocalNodeId<T>,
        mut predicate: F,
    ) -> Option<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnMut(u32, NodeType) -> bool,
    {
        let mut current_id = node_id.id;
        while let Some(parent_id) = self.parents.get_by_id(current_id) {
            let parent_type = self.tree.get_node_type(parent_id);
            if predicate(parent_id, parent_type) {
                return Some((parent_id, parent_type));
            }
            current_id = parent_id;
        }

        None
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn get_span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get_span(node_id)
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Span {
        self.source_map.get(node_id)
    }

    /// Whether the given span has a newline.
    #[inline]
    pub fn has_newline(&self, span: Span) -> bool {
        if span.start >= span.end {
            return false;
        }

        {
            let cache = self.span_has_newline_cache.borrow();
            if let Some(has_newline) = cache.get(&span) {
                if self.instrumentation_enabled {
                    self.cache_stats
                        .span_has_newline_hits
                        .set(self.cache_stats.span_has_newline_hits.get() + 1);
                }
                return *has_newline;
            }
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .span_has_newline_misses
                .set(self.cache_stats.span_has_newline_misses.get() + 1);
        }
        let newline_offsets = self.newline_offsets.get_or_init(|| {
            self.file
                .text()
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index as u32))
                .collect()
        });
        let newline_index = newline_offsets.partition_point(|offset| *offset < span.start);
        let has_newline = newline_offsets
            .get(newline_index)
            .is_some_and(|offset| *offset < span.end);
        self.span_has_newline_cache
            .borrow_mut()
            .insert(span, has_newline);
        has_newline
    }

    /// Whether the given span contains a comment token.
    #[inline]
    pub fn has_comment(&self, span: Span) -> bool {
        {
            let cache = self.span_has_comment_cache.borrow();
            if let Some(has_comment) = cache.get(&span) {
                if self.instrumentation_enabled {
                    self.cache_stats
                        .span_has_comment_hits
                        .set(self.cache_stats.span_has_comment_hits.get() + 1);
                }
                return *has_comment;
            }
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .span_has_comment_misses
                .set(self.cache_stats.span_has_comment_misses.get() + 1);
        }

        let first_relevant_index = self
            .comment_spans
            .partition_point(|comment_span| comment_span.end < span.start);

        let mut has_comment = false;
        for comment_span in &self.comment_spans[first_relevant_index..] {
            if comment_span.start > span.end {
                break;
            }

            if span.intersects(*comment_span) {
                has_comment = true;
                break;
            }
        }

        self.span_has_comment_cache
            .borrow_mut()
            .insert(span, has_comment);
        has_comment
    }

    /// Whether the given node is at a line start.
    /// (With no other semantic spans between it and the previous newline / start).
    pub fn is_at_line_start(&self, node_id: u32) -> bool {
        // find the token starting the node's span
        let span = self.get_span_by_id(node_id);
        #[cfg(debug_assertions)]
        let _span_str = self.get_span_str(span);
        let Some(mut token_idx) = self
            .tokens
            .iter()
            .position(|token| token.span.start == span.start)
        else {
            return false; // not found
        };

        // can we reach newline or start before hitting something not in side span
        while let Some(prev_token) = self.tokens.get(token_idx) {
            if token_idx == 0 || prev_token.token.ty == TokenType::Newline {
                return true; // reached start
            } else if self.side_span.contains(&prev_token.span) {
                token_idx -= 1; // keep looking
            } else {
                return false; // hit something else
            }
        }

        // reached start
        true
    }

    /// Get one formatter-owned annotation by id.
    #[inline]
    pub fn get_annotation(&self, annotation_id: LocalNodeId<Annotation>) -> Annotation {
        self.formatter_annotation_entries[annotation_id.id as usize].annotation
    }

    /// Get one formatter-owned annotation span by id.
    #[inline]
    pub fn get_annotation_span(&self, annotation_id: LocalNodeId<Annotation>) -> Span {
        self.formatter_annotation_entries[annotation_id.id as usize].span
    }

    /// Return borrowed cached annotation data for a node.
    #[inline]
    fn annotation_data_for_node<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Option<Ref<'_, CachedAnnotationData>>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let state = self.annotation_presence_cache[node_index].get();
        if state == ANNOTATION_STATE_NONE {
            if self.instrumentation_enabled {
                self.cache_stats
                    .annotation_cache_hits
                    .set(self.cache_stats.annotation_cache_hits.get() + 1);
            }
            return None;
        }

        if state == ANNOTATION_STATE_CACHED {
            if self.instrumentation_enabled {
                self.cache_stats
                    .annotation_cache_hits
                    .set(self.cache_stats.annotation_cache_hits.get() + 1);
            }
            let cache = self.annotation_ids_cache.borrow();
            return Some(Ref::map(cache, |cache| {
                cache
                    .get(node_index)
                    .and_then(|entry| entry.as_ref())
                    .expect("annotation cache should contain requested node")
            }));
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .annotation_cache_misses
                .set(self.cache_stats.annotation_cache_misses.get() + 1);
        }

        // load annotations once and cache metadata
        let annotation_ids = self
            .formatter_annotation_ids_by_node_id
            .get(node_index)
            .map(|annotation_ids| annotation_ids.as_slice().to_vec())
            .unwrap_or_default();
        let annotation_data = CachedAnnotationData::from_ids(annotation_ids, |annotation_id| {
            self.get_annotation(annotation_id)
        });
        {
            let mut cache = self.annotation_ids_cache.borrow_mut();
            cache[node_index] = Some(annotation_data);
        }
        self.annotation_presence_cache[node_index].set(ANNOTATION_STATE_CACHED);

        let cache = self.annotation_ids_cache.borrow();
        Some(Ref::map(cache, |cache| {
            cache
                .get(node_index)
                .and_then(|entry| entry.as_ref())
                .expect("annotation cache should contain requested node")
        }))
    }

    /// Get annotations for a node. Annotations are sorted by position.
    #[inline]
    pub fn get_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Option<Vec<LocalNodeId<Annotation>>>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .map(|annotation_data| annotation_data.ids.clone())
    }

    /// Read node annotations without cloning.
    #[inline]
    pub fn with_annotations<T, R, F>(&self, node_id: LocalNodeId<T>, f: F) -> Option<R>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnOnce(&[LocalNodeId<Annotation>]) -> R,
    {
        self.annotation_data_for_node(node_id)
            .map(|annotation_data| f(annotation_data.ids.as_slice()))
    }

    /// Check if a node has an annotation.
    #[inline]
    pub fn has_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let state = self.annotation_presence_cache[node_index].get();

        if !self.instrumentation_enabled {
            return state != ANNOTATION_STATE_NONE;
        }

        if state == ANNOTATION_STATE_NONE {
            self.cache_stats
                .annotation_cache_hits
                .set(self.cache_stats.annotation_cache_hits.get() + 1);
            return false;
        }
        if state == ANNOTATION_STATE_PRESENT || state == ANNOTATION_STATE_CACHED {
            self.cache_stats
                .annotation_cache_hits
                .set(self.cache_stats.annotation_cache_hits.get() + 1);
            return true;
        }
        false
    }

    /// Check if a node has a prefix annotation.
    #[inline]
    pub fn has_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_prefix)
    }

    /// Check if a node has a block infix annotation.
    #[inline]
    pub fn has_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_infix)
    }

    /// Check if a node has a non-blank annotation.
    #[inline]
    pub fn has_non_blank_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_non_blank)
    }

    /// Check if a node has a non-blank block infix annotation.
    #[inline]
    pub fn has_non_blank_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_non_blank_infix)
    }

    /// Check if a node has a postfix annotation.
    #[inline]
    pub fn has_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_postfix)
    }

    /// Check if a node has a blank block prefix annotation.
    #[inline]
    pub fn has_blank_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_blank_prefix)
    }

    /// Check if a node has a blank prefix annotation in first position.
    #[inline]
    pub fn has_blank_prefix_annotation_in_first_position<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_data_for_node(node_id)
            .is_some_and(|annotation_data| annotation_data.has_blank_prefix_first)
    }

    /// Return cached annotation facts for one argument node.
    #[inline]
    pub fn argument_annotation_profile(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> CachedArgumentAnnotationProfile {
        if let Some(profile) = self.cache_get_copy_entry(
            &self.node_caches.argument_annotation_profile,
            argument_id.id,
        ) {
            self.increment_counter("cache.argument_annotation_profile.hits", 1);
            return profile;
        }

        self.increment_counter("cache.argument_annotation_profile.misses", 1);
        let profile = self.compute_argument_annotation_profile(argument_id);
        self.cache_set_copy_entry(
            &self.node_caches.argument_annotation_profile,
            argument_id.id,
            profile,
        );

        profile
    }

    /// Return cached compact simple unannotated argument predicate.
    #[inline]
    pub fn cached_argument_compact_simple_unannotated(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> Option<bool> {
        self.cache_get_copy_entry(
            &self.node_caches.argument_compact_simple_unannotated,
            argument_id.id,
        )
    }

    /// Cache compact simple unannotated argument predicate.
    #[inline]
    pub fn cache_argument_compact_simple_unannotated(
        &self,
        argument_id: LocalNodeId<Argument>,
        value: bool,
    ) {
        self.cache_set_copy_entry(
            &self.node_caches.argument_compact_simple_unannotated,
            argument_id.id,
            value,
        );
    }

    /// Return cached plain call argument predicate.
    #[inline]
    pub fn cached_argument_plain_call_argument(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> Option<bool> {
        self.cache_get_copy_entry(
            &self.node_caches.argument_plain_call_argument,
            argument_id.id,
        )
    }

    /// Cache plain call argument predicate.
    #[inline]
    pub fn cache_argument_plain_call_argument(
        &self,
        argument_id: LocalNodeId<Argument>,
        value: bool,
    ) {
        self.cache_set_copy_entry(
            &self.node_caches.argument_plain_call_argument,
            argument_id.id,
            value,
        );
    }

    /// Return cached call argument layout-class facts for one call expression node.
    #[inline]
    pub fn cached_call_argument_layout_class(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<CachedCallArgumentLayoutClass> {
        self.cache_get_copy_entry(
            &self.node_caches.call_argument_layout_class,
            call_node_id.id,
        )
    }

    /// Cache call argument layout-class facts for one call expression node.
    #[inline]
    pub fn cache_call_argument_layout_class(
        &self,
        call_node_id: LocalNodeId<Expression>,
        layout_class: CachedCallArgumentLayoutClass,
    ) {
        self.cache_set_copy_entry(
            &self.node_caches.call_argument_layout_class,
            call_node_id.id,
            layout_class,
        );
    }

    /// Return cached call boundary-comment state for one call expression node.
    #[inline]
    pub fn cached_call_argument_boundary_comments(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<bool> {
        self.cache_get_copy_entry(
            &self.node_caches.call_argument_boundary_comments,
            call_node_id.id,
        )
    }

    /// Cache call boundary-comment state for one call expression node.
    #[inline]
    pub fn cache_call_argument_boundary_comments(
        &self,
        call_node_id: LocalNodeId<Expression>,
        has_boundary_comments: bool,
    ) {
        self.cache_set_copy_entry(
            &self.node_caches.call_argument_boundary_comments,
            call_node_id.id,
            has_boundary_comments,
        );
    }

    /// Return cached chain call force-expand decision for one call expression node.
    #[inline]
    pub fn cached_call_argument_chain_force_expand(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<bool> {
        self.cache_get_copy_entry(
            &self.node_caches.call_argument_chain_force_expand,
            call_node_id.id,
        )
    }

    /// Cache one chain call force-expand decision for one call expression node.
    #[inline]
    pub fn cache_call_argument_chain_force_expand(
        &self,
        call_node_id: LocalNodeId<Expression>,
        force_expand: bool,
    ) {
        self.cache_set_copy_entry(
            &self.node_caches.call_argument_chain_force_expand,
            call_node_id.id,
            force_expand,
        );
    }

    /// Return cached regular and chain call argument expansion profiles for one call node.
    #[inline]
    pub fn cached_call_argument_expansion_profiles(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<CachedCallArgumentExpansionProfiles> {
        self.cache_get_copy_entry(
            &self.node_caches.call_argument_expansion_profiles,
            call_node_id.id,
        )
    }

    /// Cache regular and chain call argument expansion profiles for one call node.
    #[inline]
    pub fn cache_call_argument_expansion_profiles(
        &self,
        call_node_id: LocalNodeId<Expression>,
        profiles: CachedCallArgumentExpansionProfiles,
    ) {
        self.cache_set_copy_entry(
            &self.node_caches.call_argument_expansion_profiles,
            call_node_id.id,
            profiles,
        );
    }

    /// Return cached inline call length estimate for one call node.
    #[inline]
    pub fn cached_call_inline_len_without_static_arguments(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<Option<usize>> {
        self.cache_get_copy_entry(
            &self.node_caches.call_inline_len_without_static_arguments,
            call_node_id.id,
        )
    }

    /// Cache inline call length estimate for one call node.
    #[inline]
    pub fn cache_call_inline_len_without_static_arguments(
        &self,
        call_node_id: LocalNodeId<Expression>,
        inline_len_without_static_arguments: Option<usize>,
    ) {
        self.cache_set_copy_entry(
            &self.node_caches.call_inline_len_without_static_arguments,
            call_node_id.id,
            inline_len_without_static_arguments,
        );
    }

    /// Compute annotation facts for one argument node.
    fn compute_argument_annotation_profile(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> CachedArgumentAnnotationProfile {
        let mut profile = CachedArgumentAnnotationProfile::default();
        let argument_span = self.get_span(argument_id);
        let argument_end = argument_span.end;

        // argument annotations
        if self.has_annotation(argument_id) {
            self.with_annotations(argument_id, |annotations| {
                for annotation_id in annotations {
                    let annotation = self.get_annotation(*annotation_id);

                    match annotation {
                        Annotation::Blank { .. } => {}
                        Annotation::Doc { position, .. }
                        | Annotation::Decorator { position, .. }
                        | Annotation::Comment { position, .. } => {
                            if matches!(
                                position,
                                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                            ) {
                                profile.has_prefix_annotation = true;
                            }

                            let Annotation::Comment { node, .. } = annotation else {
                                continue;
                            };
                            profile.has_comment = true;

                            let comment = self.tree.get::<Comment>(node);
                            if comment.style != ast::CommentStyle::Slash {
                                continue;
                            }

                            let annotation_span = self.get_annotation_span(*annotation_id);
                            if annotation_span.start >= argument_end {
                                profile.has_line_comment = true;
                            }
                            if matches!(
                                position,
                                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                            ) {
                                profile.has_prefix_line_comment = true;
                            }
                        }
                    }
                }
            });
        }

        // value annotations
        let argument_value_expression = match self.tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => Some(*value),
        };
        if let Some(value_id) = argument_value_expression {
            let value_id = self.transparent_inner_expression(value_id);
            let declaration_annotation_target = match self.tree.get(value_id) {
                Expression::Declaration(declaration_id) => Some(*declaration_id),
                _ => None,
            };

            // direct value annotation state
            if self.has_annotation(value_id) {
                profile.has_comment = true;
                if self.has_prefix_annotation(value_id) {
                    profile.has_prefix_annotation = true;
                }
            }

            // wrapped declaration annotation state
            if let Some(declaration_id) = declaration_annotation_target
                && self.has_annotation(declaration_id)
            {
                profile.has_comment = true;
                if self.has_prefix_annotation(declaration_id) {
                    profile.has_prefix_annotation = true;
                }
            }
        }

        profile
    }

    /// Read one copyable value from an index-addressed optional cache.
    fn cache_get_copy_entry<T: Copy>(&self, cache: &[Cell<Option<T>>], node_id: u32) -> Option<T> {
        cache.get(node_id as usize).and_then(Cell::get)
    }

    /// Write one copyable value into an index-addressed optional cache.
    fn cache_set_copy_entry<T: Copy>(&self, cache: &[Cell<Option<T>>], node_id: u32, value: T) {
        if let Some(cache_entry) = cache.get(node_id as usize) {
            cache_entry.set(Some(value));
        }
    }

    /// Start a formatter timing scope.
    #[inline]
    pub fn timing_scope(&self, tag: FormatterTimingTag) -> FormatterTimingScope {
        FormatterTimingScope::new(self.timings.as_ref(), tag)
    }

    /// Snapshot timing entries recorded by this formatter context.
    #[inline]
    pub fn timing_snapshot(&self) -> Option<Vec<FormatterTimingEntry>> {
        self.timings.as_ref().map(|timings| timings.snapshot())
    }

    /// Snapshot formatter cache counters.
    #[inline]
    pub fn cache_stats_snapshot(&self) -> FormatterCacheStatsSnapshot {
        FormatterCacheStatsSnapshot {
            span_text_hits: self.cache_stats.span_text_hits.get(),
            span_text_misses: self.cache_stats.span_text_misses.get(),
            span_has_newline_hits: self.cache_stats.span_has_newline_hits.get(),
            span_has_newline_misses: self.cache_stats.span_has_newline_misses.get(),
            span_has_comment_hits: self.cache_stats.span_has_comment_hits.get(),
            span_has_comment_misses: self.cache_stats.span_has_comment_misses.get(),
            annotation_cache_hits: self.cache_stats.annotation_cache_hits.get(),
            annotation_cache_misses: self.cache_stats.annotation_cache_misses.get(),
        }
    }

    /// Increment a formatter instrumentation counter.
    #[inline]
    pub fn increment_counter(&self, name: &'static str, delta: usize) {
        if !self.instrumentation_enabled {
            return;
        }
        self.counters.increment(name, delta);
    }

    /// Record one best_fitting evaluation for a logical formatter region.
    #[inline]
    pub fn record_best_fitting(&self, label: &'static str, variants: usize) {
        self.increment_counter("best_fitting.calls.total", 1);
        self.increment_counter("best_fitting.variants.total", variants);
        self.increment_counter(label, 1);
    }

    /// Snapshot formatter instrumentation counters.
    #[inline]
    pub fn counter_snapshot(&self) -> Vec<FormatterCounterEntry> {
        self.counters.snapshot()
    }
}

impl FormatContext for DestackFormatContext<'_> {
    type Options = DestackFormatOptions;

    #[inline]
    fn options(&self) -> &Self::Options {
        &self.options
    }

    #[inline]
    fn file(&self) -> &File {
        self.file
    }
}

/// Format Nodes with more information.
pub(crate) trait FormatNode<'a, T: Node>
where
    DestackFormatContext<'a>: FormatContext,
{
    /// Format a node.
    fn format_node(
        &self,
        node_id: LocalNodeId<T>,
        f: &mut DestackFormatter<'a, '_>,
    ) -> FormatResult<()>;
}

/// Implement Format for FormatNode for NodeIds.
impl<'a, T: Node> Format<DestackFormatContext<'a>> for LocalNodeId<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'a, '_>) -> FormatResult<()> {
        let _timing = f.context().timing_scope(tag_for_node_type(T::TYPE));
        let context = f.context();
        let node = context.tree.get(*self);
        node.format_node(*self, f)
    }
}

/// Implement Format for FormatNode for NodeIdsAny.
impl<'a> Format<DestackFormatContext<'a>> for LocalNodeIdAny {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'a, '_>) -> FormatResult<()> {
        let _timing = f.context().timing_scope(tag_for_node_type(self.ty));
        let context = f.context();
        match self.ty {
            NodeType::Expression => {
                let node_id = LocalNodeId::<Expression>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Block => {
                let node_id = LocalNodeId::<Block>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Declaration => {
                let node_id = LocalNodeId::<Declaration>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Property => {
                let node_id = LocalNodeId::<Property>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Member => {
                let node_id = LocalNodeId::<Member>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::EnumField => {
                let node_id = LocalNodeId::<EnumField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::WhereClause => {
                let node_id = LocalNodeId::<WhereClause>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::DependencyItem => {
                let node_id = LocalNodeId::<DependencyItem>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Parameter => {
                let node_id = LocalNodeId::<Parameter>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Argument => {
                let node_id = LocalNodeId::<Argument>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::MatchCase => {
                let node_id = LocalNodeId::<MatchCase>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Pattern => {
                let node_id = LocalNodeId::<Pattern>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::PatternField => {
                let node_id = LocalNodeId::<PatternField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Declarator => {
                let node_id = LocalNodeId::<Declarator>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Annotation => {
                let node_id = LocalNodeId::<Annotation>::new(self.id);
                let node = context.get_annotation(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Blank => {
                let node_id = LocalNodeId::<Blank>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Doc => {
                let node_id = LocalNodeId::<Doc>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Comment => {
                let node_id = LocalNodeId::<Comment>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Decorator => {
                let node_id = LocalNodeId::<Decorator>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
        }
    }
}
