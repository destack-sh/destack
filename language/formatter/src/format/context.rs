use std::cell::{Cell, OnceCell, Ref, RefCell};
use std::rc::Rc;

use destack_ast::{
    ANNOTATION_NODE_TYPES, Annotation as AstAnnotation, AnnotationPosition, Argument,
    AssignOperator, Blank, Block, Comment, CommentStyle, Declaration, Declarator, Decorator,
    DependencyItem, Doc, EnumField, Expression, LocalNodeId, LocalNodeIdAny, MatchCase, Member,
    Node, NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern, PatternField,
    Property, TokenSpan, TokenType, WhereClause,
};
use destack_base::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter, GroupId};
use destack_fir::print::PrintOptions;
use destack_source::{
    EnclosingSpan, File, IndentStyle, LanguageType, LineEnding, MultiSpan, NodeSearchMode,
    NodeSourceMap, Span,
};
use destack_workspace::{
    ArrowParentheses, FormatterOptions, ImportSortOrder, OrganizeImports, QuoteProperty,
    QuoteStyle, TrailingComma,
};
use rustc_hash::FxHashMap;
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
const ANNOTATION_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Newline,
    TokenType::LineComment,
    TokenType::DocLineComment,
    TokenType::BlockComment,
    TokenType::DocBlockComment,
];

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

/// Return whether one token is a side annotation token.
#[inline]
fn is_side_annotation_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::Newline
            | TokenType::LineComment
            | TokenType::DocLineComment
            | TokenType::BlockComment
            | TokenType::DocBlockComment
    )
}

/// Return true when a node id belongs to one annotation node type.
#[inline]
fn is_annotation_node_id(tree: &NodeTree, node_id: u32) -> bool {
    ANNOTATION_NODE_TYPES.contains(&tree.get_node_type(node_id))
}

/// Return true when candidate outranks current for the search mode.
#[inline]
fn is_better_enclosing_span(
    search: NodeSearchMode,
    candidate: &EnclosingSpan,
    current: &EnclosingSpan,
) -> bool {
    let candidate_len = candidate.length;
    let current_len = current.length;
    let candidate_idx = candidate.idx;
    let current_idx = current.idx;

    match search {
        NodeSearchMode::BiggestOutermost => {
            candidate_len > current_len
                || (candidate_len == current_len && candidate_idx > current_idx)
        }
        NodeSearchMode::SmallestOutermost => {
            candidate_len < current_len
                || (candidate_len == current_len && candidate_idx > current_idx)
        }
        NodeSearchMode::SmallestInnermost => {
            candidate_len < current_len
                || (candidate_len == current_len && candidate_idx < current_idx)
        }
    }
}

/// Select the best enclosing span for one range and filter.
fn select_enclosing_span_with_filter(
    source_map: &NodeSourceMap,
    start: u32,
    end_inclusive: u32,
    search: NodeSearchMode,
    filter: impl Fn(&EnclosingSpan) -> bool,
) -> Option<EnclosingSpan> {
    let mut best = None;
    source_map.visit_enclosing_spans(start, end_inclusive, |candidate| {
        if !filter(&candidate) {
            return;
        }
        match best {
            Some(current) => {
                if is_better_enclosing_span(search, &candidate, &current) {
                    best = Some(candidate);
                }
            }
            None => best = Some(candidate),
        }
    });
    best
}

/// Get the node starting at a token.
fn find_node_starting_at(
    source_map: &NodeSourceMap,
    span: &Span,
    search: NodeSearchMode,
) -> Option<EnclosingSpan> {
    select_enclosing_span_with_filter(
        source_map,
        span.start,
        span.end.saturating_sub(1),
        search,
        |candidate| candidate.span.start == span.start,
    )
}

/// Get the node ending at a token.
fn find_node_ending_at(
    source_map: &NodeSourceMap,
    span: &Span,
    search: NodeSearchMode,
) -> Option<EnclosingSpan> {
    select_enclosing_span_with_filter(
        source_map,
        span.start,
        span.end.saturating_sub(1),
        search,
        |candidate| candidate.span.end == span.end,
    )
}

/// Get the node enclosing a token.
fn find_node_enclosing_at(
    source_map: &NodeSourceMap,
    span: &Span,
    search: NodeSearchMode,
    filter: impl Fn(&EnclosingSpan) -> bool,
) -> Option<EnclosingSpan> {
    select_enclosing_span_with_filter(
        source_map,
        span.start,
        span.end.saturating_sub(1),
        search,
        filter,
    )
}

/// Collect statement wrapper ids keyed by expression id.
fn collect_statement_wrappers(tree: &NodeTree) -> Vec<Option<u32>> {
    let total_nodes = tree.next_id() as usize;
    let mut wrappers = vec![None; total_nodes];

    let mut node_id = 0usize;
    while node_id < total_nodes {
        let global_id = node_id as u32;
        if tree.get_node_type(global_id) == NodeType::Expression {
            let expression = tree.get(LocalNodeId::<Expression>::new(global_id));
            if let Expression::Statement(inner_id) = expression {
                wrappers[inner_id.id as usize] = Some(global_id);
            }
        }
        node_id += 1;
    }

    wrappers
}

/// Promote expression targets to their statement wrapper when required.
fn annotation_promote_statement(
    tree: &NodeTree,
    start_token: TokenSpan,
    target_node_id: u32,
    statement_wrappers: &[Option<u32>],
) -> u32 {
    if start_token.token.ty == TokenType::Newline
        && tree.get_node_type(target_node_id) == NodeType::Expression
        && let Some(statement_id) = statement_wrappers
            .get(target_node_id as usize)
            .and_then(|id| *id)
    {
        return statement_id;
    }

    target_node_id
}

/// Return whether a block comment stays on a single line.
fn annotation_is_single_line_block_comment(file: &File, start_token: TokenSpan) -> bool {
    let raw = file.span_str(start_token.span);
    for byte in raw.as_bytes() {
        if *byte == b'\n' || *byte == b'\r' {
            return false;
        }
    }
    true
}

/// Find the previous targetable token within the enclosing span.
fn annotation_prev_token(
    token_idx: u32,
    tokens: &[TokenSpan],
    ignore_span: &MultiSpan,
    enclosing_span: Option<Span>,
) -> Option<(usize, TokenSpan)> {
    if token_idx == 0 {
        return None;
    }

    let mut prev_token_idx = token_idx as usize;
    while prev_token_idx > 0 {
        prev_token_idx -= 1;
        let prev_token = tokens.get(prev_token_idx)?;
        if ignore_span.contains(&prev_token.span) || prev_token.token.ty == TokenType::Whitespace {
            continue;
        }

        if let Some(enclosing_span) = enclosing_span
            && !enclosing_span.intersects(prev_token.span)
        {
            return None;
        }

        return Some((prev_token_idx, *prev_token));
    }

    None
}

/// Find the next targetable token within the enclosing span.
fn annotation_next_token(
    token_idx: u32,
    group_len: usize,
    tokens: &[TokenSpan],
    ignore_span: &MultiSpan,
    enclosing_span: Option<Span>,
) -> Option<(usize, TokenSpan)> {
    let mut next_token_idx = token_idx as usize + group_len;
    loop {
        let next_token = tokens.get(next_token_idx)?;
        if ignore_span.contains(&next_token.span) || next_token.token.ty == TokenType::Whitespace {
            next_token_idx += 1;
            continue;
        }

        if let Some(enclosing_span) = enclosing_span
            && !enclosing_span.intersects(next_token.span)
        {
            return None;
        }

        return Some((next_token_idx, *next_token));
    }
}

/// Find the next non-annotation token from one token index.
fn annotation_next_non_annotation_token(
    token_idx: usize,
    tokens: &[TokenSpan],
    ignore_span: &MultiSpan,
    enclosing_span: Option<Span>,
) -> Option<(usize, TokenSpan)> {
    let mut next_token_idx = token_idx;
    loop {
        let next_token = tokens.get(next_token_idx)?;
        if ignore_span.contains(&next_token.span)
            || next_token.token.ty == TokenType::Whitespace
            || ANNOTATION_TOKEN_TYPES.contains(&next_token.token.ty)
        {
            next_token_idx += 1;
            continue;
        }

        if let Some(enclosing_span) = enclosing_span
            && !enclosing_span.intersects(next_token.span)
        {
            return None;
        }

        return Some((next_token_idx, *next_token));
    }
}

/// Find the previous significant token, excluding whitespace, newlines, and annotation tokens.
fn annotation_prev_significant_token(
    token_idx: usize,
    tokens: &[TokenSpan],
    ignore_span: &MultiSpan,
    enclosing_span: Option<Span>,
) -> Option<TokenSpan> {
    let mut cursor = token_idx;
    while cursor > 0 {
        cursor -= 1;
        let token = tokens.get(cursor)?;
        if ignore_span.contains(&token.span)
            || token.token.ty == TokenType::Whitespace
            || token.token.ty == TokenType::Newline
            || ANNOTATION_TOKEN_TYPES.contains(&token.token.ty)
        {
            continue;
        }

        if let Some(enclosing_span) = enclosing_span
            && !enclosing_span.intersects(token.span)
        {
            return None;
        }

        return Some(*token);
    }

    None
}

/// Resolve one argument node id to its value expression node id.
fn annotation_argument_value_expression_id(tree: &NodeTree, node_id: u32) -> Option<u32> {
    if tree.get_node_type(node_id) != NodeType::Argument {
        return None;
    }

    let argument_id = LocalNodeId::<Argument>::new(node_id);
    let value_id = match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };
    Some(value_id.id)
}

/// Find one call/new owner for one argument-list open parenthesis token.
fn find_call_like_owner_for_open_parenthesis(
    tree: &NodeTree,
    source_map: &NodeSourceMap,
    open_parenthesis_token: TokenSpan,
    ignore_span: &MultiSpan,
) -> Option<u32> {
    if open_parenthesis_token.token.ty != TokenType::OpenParenthesis {
        return None;
    }

    let call_owner_id = find_node_enclosing_at(
        source_map,
        &open_parenthesis_token.span,
        NodeSearchMode::SmallestOutermost,
        |candidate| {
            if is_annotation_node_id(tree, candidate.idx) || ignore_span.contains(&candidate.span) {
                return false;
            }

            if tree.get_node_type(candidate.idx) != NodeType::Expression {
                return false;
            }

            matches!(
                tree.get(LocalNodeId::<Expression>::new(candidate.idx)),
                Expression::Call { .. } | Expression::New { .. }
            )
        },
    )
    .map(|span| span.idx)?;

    // ensure this `(` belongs to the call/new argument list, not an inner parenthesized expression
    let call_owner_expression_id = LocalNodeId::<Expression>::new(call_owner_id);
    let call_left_id = match tree.get(call_owner_expression_id) {
        Expression::Call { left, .. } | Expression::New { left, .. } => *left,
        _ => return None,
    };

    let call_span = tree.get_span(call_owner_expression_id);
    let call_left_span = tree.get_span(call_left_id);
    let open_start = open_parenthesis_token.span.start;
    if open_start < call_left_span.end || open_start >= call_span.end {
        return None;
    }

    Some(call_owner_id)
}

/// Resolve one call-like argument seam target from comment neighbors.
fn resolve_call_like_argument_seam_target(
    tree: &NodeTree,
    source_map: &NodeSourceMap,
    prev_token: Option<TokenSpan>,
    next_token: Option<TokenSpan>,
    ignore_span: &MultiSpan,
) -> Option<(AnnotationPosition, u32)> {
    // comment between callee and argument list: `callee /* note */ (arg)`
    if let Some(open_parenthesis_token) = next_token
        && open_parenthesis_token.token.ty == TokenType::OpenParenthesis
        && let Some(call_owner_id) = find_call_like_owner_for_open_parenthesis(
            tree,
            source_map,
            open_parenthesis_token,
            ignore_span,
        )
    {
        let call_owner_expression_id = LocalNodeId::<Expression>::new(call_owner_id);
        let dynamic_arguments = match tree.get(call_owner_expression_id) {
            Expression::Call {
                dynamic_arguments, ..
            }
            | Expression::New {
                dynamic_arguments, ..
            } => dynamic_arguments,
            _ => return None,
        };

        if let Some(first_argument_id) = dynamic_arguments.first().copied() {
            let target_id = annotation_argument_value_expression_id(tree, first_argument_id.id)
                .unwrap_or(first_argument_id.id);
            return Some((AnnotationPosition::LinePrefix, target_id));
        }

        return Some((AnnotationPosition::BlockInfix, call_owner_id));
    }

    // comment at argument list start: `callee( /* note */ arg )` or `callee( // note\n )`
    if let Some(open_parenthesis_token) = prev_token
        && open_parenthesis_token.token.ty == TokenType::OpenParenthesis
        && let Some(call_owner_id) = find_call_like_owner_for_open_parenthesis(
            tree,
            source_map,
            open_parenthesis_token,
            ignore_span,
        )
    {
        let call_owner_expression_id = LocalNodeId::<Expression>::new(call_owner_id);
        let dynamic_arguments = match tree.get(call_owner_expression_id) {
            Expression::Call {
                dynamic_arguments, ..
            }
            | Expression::New {
                dynamic_arguments, ..
            } => dynamic_arguments,
            _ => return None,
        };

        if let Some(first_argument_id) = dynamic_arguments.first().copied() {
            let target_id = annotation_argument_value_expression_id(tree, first_argument_id.id)
                .unwrap_or(first_argument_id.id);
            return Some((AnnotationPosition::LinePrefix, target_id));
        }

        return Some((AnnotationPosition::BlockInfix, call_owner_id));
    }

    None
}

/// Return whether one token keeps separator comments as line-prefix on the following node.
fn annotation_is_forward_prefix_separator(token: TokenSpan) -> bool {
    token.token.ty == TokenType::Colon || AssignOperator::from_token(token.token.ty).is_some()
}

/// Return whether a token is one identifier keyword.
#[inline]
fn annotation_is_identifier_keyword(file: &File, token: TokenSpan, keyword: &str) -> bool {
    token.token.ty == TokenType::Identifier && file.span_str(token.span) == keyword
}

/// Return whether this separator token is `as` or `satisfies`.
#[inline]
fn annotation_is_type_operator_separator(file: &File, token: TokenSpan) -> bool {
    annotation_is_identifier_keyword(file, token, "as")
        || annotation_is_identifier_keyword(file, token, "satisfies")
}

/// Return whether this separator seam targets `as const` or `as comptime`.
#[inline]
fn annotation_is_type_unary_as_keyword_target(file: &File, token: TokenSpan) -> bool {
    annotation_is_identifier_keyword(file, token, "const")
        || annotation_is_identifier_keyword(file, token, "comptime")
}

/// Return whether a token starts one declaration head keyword.
#[inline]
fn annotation_is_declaration_head_keyword(file: &File, token: TokenSpan) -> bool {
    token.token.ty == TokenType::Identifier
        && matches!(
            file.span_str(token.span),
            "class"
                | "struct"
                | "enum"
                | "interface"
                | "type"
                | "newtype"
                | "function"
                | "namespace"
                | "module"
                | "global"
                | "extension"
                | "import"
        )
}

/// Return whether a token is one declaration-head modifier keyword.
#[inline]
fn annotation_is_declaration_head_modifier(file: &File, token: TokenSpan) -> bool {
    annotation_is_identifier_keyword(file, token, "declare")
        || annotation_is_identifier_keyword(file, token, "abstract")
        || annotation_is_identifier_keyword(file, token, "async")
        || annotation_is_identifier_keyword(file, token, "default")
}

/// Return whether one token index starts one declaration head including optional modifiers.
fn annotation_is_declaration_head_start(
    file: &File,
    tokens: &[TokenSpan],
    start_idx: usize,
    ignore_span: &MultiSpan,
    enclosing_span: Option<Span>,
) -> bool {
    let mut cursor = start_idx;
    loop {
        let Some((token_idx, token)) =
            annotation_next_non_annotation_token(cursor, tokens, ignore_span, enclosing_span)
        else {
            return false;
        };

        if annotation_is_declaration_head_keyword(file, token) {
            return true;
        }
        if !annotation_is_declaration_head_modifier(file, token) {
            return false;
        }

        cursor = token_idx + 1;
    }
}

/// Find one type-operator expression owner for comments on `as`/`satisfies` seams.
fn find_type_operator_owner_for_separator(
    tree: &NodeTree,
    source_map: &NodeSourceMap,
    prev_token: TokenSpan,
    forward_token: TokenSpan,
    ignore_span: &MultiSpan,
) -> Option<u32> {
    find_node_enclosing_at(
        source_map,
        &prev_token.span,
        NodeSearchMode::SmallestOutermost,
        |candidate| {
            if is_annotation_node_id(tree, candidate.idx) || ignore_span.contains(&candidate.span) {
                return false;
            }

            if tree.get_node_type(candidate.idx) != NodeType::Expression {
                return false;
            }

            if !candidate.span.intersects(prev_token.span)
                || !candidate.span.intersects(forward_token.span)
            {
                return false;
            }

            matches!(
                tree.get(LocalNodeId::<Expression>::new(candidate.idx)),
                Expression::TypeBinary {
                    operator: destack_ast::TypeBinaryOperator::Cast
                        | destack_ast::TypeBinaryOperator::Satisfies,
                    ..
                } | Expression::TypeUnary {
                    operator: destack_ast::TypeUnaryOperator::AsConst
                        | destack_ast::TypeUnaryOperator::AsComptime,
                    ..
                }
            )
        },
    )
    .map(|span| span.idx)
}

/// Resolve one `satisfies` right side to its first static argument target when list has multiple arguments.
fn find_satisfies_right_static_argument_target(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return None;
    }

    let owner_expression_id = LocalNodeId::<Expression>::new(owner_id);
    let Expression::TypeBinary {
        operator: destack_ast::TypeBinaryOperator::Satisfies,
        right,
        ..
    } = tree.get(owner_expression_id)
    else {
        return None;
    };

    let right_id = match tree.get(*right) {
        Expression::Parenthesized { expression } | Expression::Statement(expression) => *expression,
        _ => *right,
    };

    let static_arguments = match tree.get(right_id) {
        Expression::Path {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        }
        | Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        }
        | Expression::TypeImport {
            static_arguments, ..
        } => static_arguments.as_ref(),
        Expression::Instantiation {
            static_arguments, ..
        } => Some(static_arguments),
        _ => None,
    }?;

    if static_arguments.len() <= 1 {
        return None;
    }

    Some(static_arguments[0].id)
}

/// Return the last extends expression id for one declaration when present.
fn declaration_last_extends_expression(tree: &NodeTree, declaration_id: u32) -> Option<u32> {
    let declaration_id = LocalNodeId::<Declaration>::new(declaration_id);
    let extends_types = match tree.get(declaration_id) {
        Declaration::Struct { heritage, .. }
        | Declaration::Class { heritage, .. }
        | Declaration::Enum { heritage, .. }
        | Declaration::Interface { heritage, .. }
        | Declaration::Extension { heritage, .. } => heritage.extends_types.as_ref(),
        _ => None,
    }?;

    extends_types.last().map(|expression_id| expression_id.id)
}

/// Return whether one declaration heritage references this expression id.
fn declaration_heritage_contains_expression(
    tree: &NodeTree,
    declaration_id: u32,
    expression_id: u32,
) -> bool {
    let declaration_id = LocalNodeId::<Declaration>::new(declaration_id);
    let (extends_types, implements_types) = match tree.get(declaration_id) {
        Declaration::Struct { heritage, .. }
        | Declaration::Class { heritage, .. }
        | Declaration::Enum { heritage, .. }
        | Declaration::Interface { heritage, .. }
        | Declaration::Extension { heritage, .. } => (
            heritage.extends_types.as_deref().unwrap_or_default(),
            heritage.implements_types.as_deref().unwrap_or_default(),
        ),
        _ => return false,
    };

    extends_types
        .iter()
        .chain(implements_types.iter())
        .any(|expression| expression.id == expression_id)
}

/// Return the body attachment target for comments that sit right before declaration `{`.
fn declaration_body_comment_target(
    tree: &NodeTree,
    declaration_id: u32,
) -> Option<(AnnotationPosition, u32)> {
    let declaration_id = LocalNodeId::<Declaration>::new(declaration_id);

    match tree.get(declaration_id) {
        Declaration::Struct { members, .. }
        | Declaration::Class { members, .. }
        | Declaration::Interface { members, .. }
        | Declaration::Extension { members, .. } => members
            .first()
            .map(|member_id| (AnnotationPosition::BlockPrefix, member_id.id))
            .or(Some((AnnotationPosition::BlockInfix, declaration_id.id))),
        Declaration::Enum {
            fields, members, ..
        } => fields
            .first()
            .map(|field_id| (AnnotationPosition::BlockPrefix, field_id.id))
            .or_else(|| {
                members
                    .first()
                    .map(|member_id| (AnnotationPosition::BlockPrefix, member_id.id))
            })
            .or(Some((AnnotationPosition::BlockInfix, declaration_id.id))),
        Declaration::Namespace { expressions, .. } | Declaration::Global { expressions, .. } => {
            expressions
                .first()
                .map(|expression_id| (AnnotationPosition::BlockPrefix, expression_id.id))
                .or(Some((AnnotationPosition::BlockInfix, declaration_id.id)))
        }
        _ => None,
    }
}

/// Return whether this owner node is field-like and can absorb field-tail comments.
fn annotation_owner_is_field_like(tree: &NodeTree, owner_id: u32) -> bool {
    match tree.get_node_type(owner_id) {
        NodeType::Member => matches!(
            tree.get(LocalNodeId::<Member>::new(owner_id)),
            Member::Field { .. } | Member::Type { .. } | Member::ComptimeConst { .. }
        ),
        NodeType::Property => matches!(
            tree.get(LocalNodeId::<Property>::new(owner_id)),
            Property::Field { .. }
        ),
        _ => false,
    }
}

/// Return whether one `:` separator owns a mapped-type value seam.
fn mapped_type_value_target_for_separator(
    tree: &NodeTree,
    source_map: &NodeSourceMap,
    prev_token: TokenSpan,
    forward_token: TokenSpan,
    forward_target_id: u32,
    ignore_span: &MultiSpan,
) -> Option<u32> {
    let mapped_owner_id = find_node_enclosing_at(
        source_map,
        &prev_token.span,
        NodeSearchMode::SmallestOutermost,
        |candidate| {
            if is_annotation_node_id(tree, candidate.idx) || ignore_span.contains(&candidate.span) {
                return false;
            }

            if tree.get_node_type(candidate.idx) != NodeType::Expression {
                return false;
            }

            if !candidate.span.intersects(prev_token.span)
                || !candidate.span.intersects(forward_token.span)
            {
                return false;
            }

            matches!(
                tree.get(LocalNodeId::<Expression>::new(candidate.idx)),
                Expression::TypeMapped { .. }
            )
        },
    )
    .map(|span| span.idx)?;

    let mapped_owner_id = LocalNodeId::<Expression>::new(mapped_owner_id);
    let Expression::TypeMapped { value, .. } = tree.get(mapped_owner_id) else {
        return None;
    };

    if forward_target_id == value.id {
        return Some(value.id);
    }

    None
}

/// Return whether this `)` token closes a control head like `if (...)` or `while (...)`.
fn annotation_is_control_head_close_parenthesis(
    file: &File,
    tokens: &[TokenSpan],
    close_parenthesis_idx: usize,
) -> bool {
    let close_parenthesis = tokens.get(close_parenthesis_idx);
    if close_parenthesis.is_none_or(|token| token.token.ty != TokenType::CloseParenthesis) {
        return false;
    }

    let mut depth = 0u32;
    let mut open_parenthesis_idx = None;
    let mut cursor = close_parenthesis_idx;
    while cursor > 0 {
        cursor -= 1;
        let token = tokens[cursor];
        if token.token.ty == TokenType::CloseParenthesis {
            depth += 1;
            continue;
        }
        if token.token.ty == TokenType::OpenParenthesis {
            if depth == 0 {
                open_parenthesis_idx = Some(cursor);
                break;
            }
            depth -= 1;
        }
    }

    let Some(open_parenthesis_idx) = open_parenthesis_idx else {
        return false;
    };

    let mut keyword_idx = open_parenthesis_idx;
    while keyword_idx > 0 {
        keyword_idx -= 1;
        let token = tokens[keyword_idx];
        if token.token.ty == TokenType::Whitespace || token.token.ty == TokenType::Newline {
            continue;
        }
        if ANNOTATION_TOKEN_TYPES.contains(&token.token.ty) {
            continue;
        }

        let keyword_text = file.span_str(token.span);
        return matches!(
            keyword_text,
            "if" | "while" | "for" | "switch" | "catch" | "with"
        );
    }

    false
}

/// Find the annotation target for one side annotation token group.
#[allow(clippy::too_many_arguments)]
fn find_side_annotation_target(
    tree: &NodeTree,
    source_map: &NodeSourceMap,
    file: &File,
    token_idx: u32,
    tokens: &[TokenSpan],
    line_indices: &[u32],
    group_start_idx: usize,
    group_end_idx: usize,
    group_len: usize,
    is_full_line: bool,
    is_block_prefix_only: bool,
    statement_wrappers: &[Option<u32>],
    ignore_span: &MultiSpan,
) -> Option<(AnnotationPosition, u32)> {
    debug_assert!(group_len > 0);

    let debug_trivia = std::env::var("DESTACK_DEBUG_TRIVIA").is_ok();
    let start_token = tokens[group_start_idx];
    let is_block_comment = matches!(
        start_token.token.ty,
        TokenType::BlockComment | TokenType::DocBlockComment
    );
    let is_block_comment_single_line = if is_block_comment {
        annotation_is_single_line_block_comment(file, start_token)
    } else {
        false
    };
    let is_one_line = if is_block_comment && !is_block_comment_single_line {
        false
    } else {
        line_indices[group_start_idx] == line_indices[group_end_idx]
    };
    let enclosing_scope = find_node_enclosing_at(
        source_map,
        &start_token.span,
        NodeSearchMode::SmallestInnermost,
        |candidate| {
            !is_annotation_node_id(tree, candidate.idx) && !ignore_span.contains(&candidate.span)
        },
    );
    let enclosing_span = enclosing_scope.map(|scope| scope.span);

    // type operator separator seams
    if !is_block_prefix_only {
        let prev_token = annotation_prev_token(token_idx, tokens, ignore_span, enclosing_span);
        let forward_token = annotation_next_non_annotation_token(
            token_idx as usize + group_len,
            tokens,
            ignore_span,
            enclosing_span,
        );

        // `as const` and `as comptime`: seam comments belong to the full expression boundary
        if let (Some((_, prev_token)), Some((_, forward_token))) = (prev_token, forward_token)
            && annotation_is_identifier_keyword(file, prev_token, "as")
            && annotation_is_type_unary_as_keyword_target(file, forward_token)
            && let Some(operator_owner_id) = find_type_operator_owner_for_separator(
                tree,
                source_map,
                prev_token,
                forward_token,
                ignore_span,
            )
        {
            let target_node_id = annotation_promote_statement(
                tree,
                start_token,
                operator_owner_id,
                statement_wrappers,
            );
            return Some((AnnotationPosition::LinePostfixBoundary, target_node_id));
        }

        // `as` and `satisfies` with line comments before the rhs: keep on expression boundary
        if start_token.token.ty == TokenType::LineComment
            && let (Some((_, prev_token)), Some((_, forward_token))) = (prev_token, forward_token)
            && annotation_is_type_operator_separator(file, prev_token)
            && !annotation_is_type_unary_as_keyword_target(file, forward_token)
            && let Some(operator_owner_id) = find_type_operator_owner_for_separator(
                tree,
                source_map,
                prev_token,
                forward_token,
                ignore_span,
            )
        {
            if let Some(argument_target_id) =
                find_satisfies_right_static_argument_target(tree, operator_owner_id)
            {
                return Some((AnnotationPosition::LinePrefix, argument_target_id));
            }

            let target_node_id = annotation_promote_statement(
                tree,
                start_token,
                operator_owner_id,
                statement_wrappers,
            );
            return Some((AnnotationPosition::LinePostfixBoundary, target_node_id));
        }

        // `export // comment` before declaration heads: keep comment between export and head
        if start_token.token.ty == TokenType::LineComment
            && let (Some((prev_idx, prev_token)), Some((forward_idx, forward_token))) =
                (prev_token, forward_token)
            && line_indices[prev_idx] == line_indices[group_end_idx]
            && annotation_is_identifier_keyword(file, prev_token, "export")
            && annotation_is_declaration_head_start(
                file,
                tokens,
                forward_idx,
                ignore_span,
                enclosing_span,
            )
            && let Some(declaration_owner_id) = find_node_enclosing_at(
                source_map,
                &prev_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| {
                    !is_annotation_node_id(tree, candidate.idx)
                        && tree.get_node_type(candidate.idx) == NodeType::Declaration
                        && candidate.span.intersects(forward_token.span)
                },
            )
            .map(|span| span.idx)
        {
            return Some((AnnotationPosition::LinePrefix, declaration_owner_id));
        }
    }

    // line prefix or postfix
    if !is_block_prefix_only && is_one_line {
        let prev_token = annotation_prev_token(token_idx, tokens, ignore_span, enclosing_span);
        let next_token =
            annotation_next_token(token_idx, group_len, tokens, ignore_span, enclosing_span);

        // call/new argument seams: keep comments in argument list context
        let prev_seam_token = prev_token.and_then(|(prev_idx, prev_token)| {
            if prev_token.token.ty == TokenType::Newline {
                return annotation_prev_significant_token(
                    prev_idx,
                    tokens,
                    ignore_span,
                    enclosing_span,
                );
            }

            Some(prev_token)
        });
        if let Some((position, target_id)) = resolve_call_like_argument_seam_target(
            tree,
            source_map,
            prev_seam_token,
            next_token.map(|(_, token)| token),
            ignore_span,
        ) {
            return Some((position, target_id));
        }

        // assignment seams with single-line block comments before a newline:
        // keep the marker on the rhs prefix instead of trailing the lhs statement
        if is_block_comment_single_line
            && let Some((prev_idx, prev_token)) = prev_token
            && line_indices[prev_idx] == line_indices[group_end_idx]
            && AssignOperator::from_token(prev_token.token.ty).is_some()
            && next_token.is_some_and(|(_, token)| token.token.ty == TokenType::Newline)
            && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                token_idx as usize + group_len,
                tokens,
                ignore_span,
                enclosing_span,
            )
            && let Some(forward_target_id) = find_node_starting_at(
                source_map,
                &forward_token.span,
                NodeSearchMode::SmallestOutermost,
            )
            .or_else(|| {
                find_node_enclosing_at(
                    source_map,
                    &forward_token.span,
                    NodeSearchMode::SmallestOutermost,
                    |candidate| !is_annotation_node_id(tree, candidate.idx),
                )
            })
            .map(|span| span.idx)
        {
            return Some((
                AnnotationPosition::BlockPrefix,
                annotation_promote_statement(
                    tree,
                    start_token,
                    forward_target_id,
                    statement_wrappers,
                ),
            ));
        }

        // assignment seam line comments on their own line:
        // keep as rhs line-prefix so surrounding blank lines stay stable
        if start_token.token.ty == TokenType::LineComment
            && let Some(previous_significant_token) = annotation_prev_significant_token(
                token_idx as usize,
                tokens,
                ignore_span,
                enclosing_span,
            )
            && AssignOperator::from_token(previous_significant_token.token.ty).is_some()
            && next_token.is_some_and(|(_, token)| token.token.ty == TokenType::Newline)
            && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                token_idx as usize + group_len,
                tokens,
                ignore_span,
                enclosing_span,
            )
            && let Some(forward_target_id) = find_node_starting_at(
                source_map,
                &forward_token.span,
                NodeSearchMode::SmallestOutermost,
            )
            .or_else(|| {
                find_node_enclosing_at(
                    source_map,
                    &forward_token.span,
                    NodeSearchMode::SmallestOutermost,
                    |candidate| !is_annotation_node_id(tree, candidate.idx),
                )
            })
            .map(|span| span.idx)
        {
            return Some((
                AnnotationPosition::LinePrefix,
                annotation_promote_statement(
                    tree,
                    start_token,
                    forward_target_id,
                    statement_wrappers,
                ),
            ));
        }

        // inline member split: keep comments between receiver and dot on their own line
        if let (Some((prev_idx, _)), Some((next_idx, next_token)), Some(enclosing_scope)) =
            (prev_token, next_token, enclosing_scope)
            && next_token.token.ty == TokenType::Dot
            && line_indices[group_end_idx] == line_indices[next_idx]
            && line_indices[prev_idx] == line_indices[group_end_idx]
            && tree.get_node_type(enclosing_scope.idx) == NodeType::Expression
        {
            let enclosing_expr_id = LocalNodeId::<Expression>::new(enclosing_scope.idx);
            if matches!(tree.get(enclosing_expr_id), Expression::Path { .. }) {
                return Some((AnnotationPosition::BlockInfix, enclosing_scope.idx));
            }
        }

        // line postfix: check for directly preceding node that ends at the start token
        let line_postfix_target = if let Some((prev_idx, prev_token)) = prev_token
            && line_indices[prev_idx] == line_indices[group_end_idx]
            && prev_token.token.ty != TokenType::Newline
        {
            let search_mode = if is_full_line && prev_token.token.ty == TokenType::Comma {
                NodeSearchMode::BiggestOutermost
            } else {
                NodeSearchMode::SmallestOutermost
            };
            find_node_ending_at(source_map, &prev_token.span, search_mode)
                .or_else(|| {
                    // if prev_token is a separator and comment is at end of line,
                    // look past the separator to find the element (handles `3, // comment`)
                    let is_end_of_line = next_token.is_none()
                        || next_token.unwrap().1.token.ty == TokenType::Newline
                        || next_token.unwrap().1.token.ty == TokenType::End;
                    if is_end_of_line
                        && matches!(
                            prev_token.token.ty,
                            TokenType::Comma | TokenType::Semicolon | TokenType::ElementwiseOr
                        )
                    {
                        // semicolon seam comments belong to the enclosing statement or member
                        if prev_token.token.ty == TokenType::Semicolon {
                            if let Some(separator_owner) = find_node_enclosing_at(
                                source_map,
                                &prev_token.span,
                                NodeSearchMode::SmallestOutermost,
                                |candidate| {
                                    !is_annotation_node_id(tree, candidate.idx)
                                        && !ignore_span.contains(&candidate.span)
                                },
                            ) {
                                return Some(separator_owner);
                            }
                        }

                        if token_idx <= 1 {
                            return None;
                        }

                        let mut before_sep_idx = token_idx as usize - 1;
                        while before_sep_idx > 0 {
                            before_sep_idx -= 1;
                            let before_token = tokens.get(before_sep_idx)?;
                            if ignore_span.contains(&before_token.span)
                                || before_token.token.ty == TokenType::Whitespace
                            {
                                continue;
                            }

                            return find_node_ending_at(
                                source_map,
                                &before_token.span,
                                search_mode,
                            );
                        }
                    }
                    None
                })
                .map(|span| span.idx)
        } else {
            None
        };

        if let Some(target_node_id) = line_postfix_target {
            if next_token.is_none()
                || next_token.unwrap().1.token.ty == TokenType::Newline
                || next_token.unwrap().1.token.ty == TokenType::End
            {
                // control-head line comments like `if (x) // note` bind to the body
                if start_token.token.ty == TokenType::LineComment
                    && let Some((prev_idx, prev_token)) = prev_token
                    && prev_token.token.ty == TokenType::CloseParenthesis
                    && annotation_is_control_head_close_parenthesis(file, tokens, prev_idx)
                    && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                        token_idx as usize + group_len,
                        tokens,
                        ignore_span,
                        enclosing_span,
                    )
                    && let Some(forward_target_id) = find_node_starting_at(
                        source_map,
                        &forward_token.span,
                        NodeSearchMode::SmallestInnermost,
                    )
                    .or_else(|| {
                        find_node_enclosing_at(
                            source_map,
                            &forward_token.span,
                            NodeSearchMode::SmallestInnermost,
                            |candidate| !is_annotation_node_id(tree, candidate.idx),
                        )
                    })
                    .map(|span| span.idx)
                {
                    if debug_trivia {
                        eprintln!("target-debug: control-head -> line-prefix {forward_target_id}");
                    }
                    let mut forward_target_id = forward_target_id;

                    // prefer the widest expression starting at the seam so `run` promotes to `run()`
                    if tree.get_node_type(forward_target_id) == NodeType::Expression
                        && let Some(outer_expression_id) = find_node_starting_at(
                            source_map,
                            &forward_token.span,
                            NodeSearchMode::BiggestOutermost,
                        )
                        .map(|span| span.idx)
                        .filter(|candidate_id| {
                            tree.get_node_type(*candidate_id) == NodeType::Expression
                        })
                    {
                        forward_target_id = outer_expression_id;
                    }

                    // unwrap implicit body blocks to their single statement expression
                    if tree.get_node_type(forward_target_id) == NodeType::Expression {
                        let expression_id = LocalNodeId::<Expression>::new(forward_target_id);
                        if let Expression::Block(block_id) = tree.get(expression_id) {
                            let block = tree.get::<Block>(*block_id);
                            if block.format == destack_ast::BlockFormat::Implicit
                                && block.expressions.len() == 1
                            {
                                forward_target_id = block.expressions[0].id;
                            }
                        }
                    }

                    let promoted_target_id =
                        if tree.get_node_type(forward_target_id) == NodeType::Expression {
                            statement_wrappers
                                .get(forward_target_id as usize)
                                .and_then(|id| *id)
                                .unwrap_or(forward_target_id)
                        } else {
                            forward_target_id
                        };
                    return Some((AnnotationPosition::LinePrefix, promoted_target_id));
                }

                // continuation seams like `target // note` + newline + `?.()`
                if start_token.token.ty == TokenType::LineComment
                    && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                        token_idx as usize + group_len,
                        tokens,
                        ignore_span,
                        enclosing_span,
                    )
                    && matches!(
                        forward_token.token.ty,
                        TokenType::Maybe | TokenType::OpenParenthesis | TokenType::OpenBracket
                    )
                    && let Some(continuation_owner_id) = find_node_enclosing_at(
                        source_map,
                        &forward_token.span,
                        NodeSearchMode::SmallestOutermost,
                        |candidate| !is_annotation_node_id(tree, candidate.idx),
                    )
                    .map(|span| span.idx)
                    && continuation_owner_id != target_node_id
                    && tree.get_node_type(continuation_owner_id) == NodeType::Expression
                {
                    if debug_trivia {
                        eprintln!(
                            "target-debug: continuation promotion {target_node_id} -> {continuation_owner_id}"
                        );
                    }
                    return Some((AnnotationPosition::LinePostfix, continuation_owner_id));
                }

                // declaration and method heads: move `// comment` before `{` into block bodies
                if start_token.token.ty == TokenType::LineComment
                    && let Some((_, prev_token)) = prev_token
                    && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                        token_idx as usize + group_len,
                        tokens,
                        ignore_span,
                        enclosing_span,
                    )
                    && forward_token.token.ty == TokenType::OpenBrace
                    && let Some(declaration_owner_id) = find_node_enclosing_at(
                        source_map,
                        &prev_token.span,
                        NodeSearchMode::SmallestOutermost,
                        |candidate| {
                            !is_annotation_node_id(tree, candidate.idx)
                                && tree.get_node_type(candidate.idx) == NodeType::Declaration
                        },
                    )
                    .map(|span| span.idx)
                    && declaration_heritage_contains_expression(
                        tree,
                        declaration_owner_id,
                        target_node_id,
                    )
                    && let Some((position, target_id)) =
                        declaration_body_comment_target(tree, declaration_owner_id)
                {
                    return Some((position, target_id));
                }

                // expression and method heads: move `// comment` before `{` into block bodies
                if start_token.token.ty == TokenType::LineComment
                    && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                        token_idx as usize + group_len,
                        tokens,
                        ignore_span,
                        enclosing_span,
                    )
                    && forward_token.token.ty == TokenType::OpenBrace
                    && let Some(forward_target_id) = find_node_starting_at(
                        source_map,
                        &forward_token.span,
                        NodeSearchMode::SmallestOutermost,
                    )
                    .or_else(|| {
                        find_node_enclosing_at(
                            source_map,
                            &forward_token.span,
                            NodeSearchMode::SmallestOutermost,
                            |candidate| !is_annotation_node_id(tree, candidate.idx),
                        )
                    })
                    .map(|span| span.idx)
                {
                    if tree.get_node_type(forward_target_id) == NodeType::Expression {
                        let forward_expression_id =
                            LocalNodeId::<Expression>::new(forward_target_id);
                        if let Expression::Block(block_id) = tree.get(forward_expression_id) {
                            let block = tree.get::<Block>(*block_id);
                            if let Some(first_expression_id) = block.expressions.first().copied() {
                                return Some((
                                    AnnotationPosition::BlockPrefix,
                                    first_expression_id.id,
                                ));
                            }

                            return Some((AnnotationPosition::BlockInfix, forward_target_id));
                        }
                    }

                    if tree.get_node_type(forward_target_id) == NodeType::Declaration
                        && let Some((position, target_id)) =
                            declaration_body_comment_target(tree, forward_target_id)
                    {
                        return Some((position, target_id));
                    }
                }

                // field-like tails: keep boundary comments on member/property owners
                if tree.get_node_type(target_node_id) == NodeType::Expression
                    && statement_wrappers
                        .get(target_node_id as usize)
                        .and_then(|id| *id)
                        .is_none()
                    && let Some((_, prev_token)) = prev_token
                    && let Some(field_owner_id) = find_node_enclosing_at(
                        source_map,
                        &prev_token.span,
                        NodeSearchMode::SmallestOutermost,
                        |candidate| {
                            !is_annotation_node_id(tree, candidate.idx)
                                && matches!(
                                    tree.get_node_type(candidate.idx),
                                    NodeType::Member | NodeType::Property
                                )
                        },
                    )
                    .map(|span| span.idx)
                    && annotation_owner_is_field_like(tree, field_owner_id)
                {
                    return Some((AnnotationPosition::LinePostfixBoundary, field_owner_id));
                }

                if debug_trivia {
                    eprintln!("target-debug: line-postfix-boundary {target_node_id}");
                }
                return Some((AnnotationPosition::LinePostfixBoundary, target_node_id));
            } else {
                if debug_trivia {
                    eprintln!("target-debug: line-postfix {target_node_id}");
                }
                return Some((AnnotationPosition::LinePostfix, target_node_id));
            }
        }
        // separator seams like `name: // comment` and `lhs = // marker` stay on rhs line-prefix
        else if start_token.token.ty == TokenType::LineComment
            && let Some((prev_idx, prev_token)) = prev_token
            && line_indices[prev_idx] == line_indices[group_end_idx]
            && annotation_is_forward_prefix_separator(prev_token)
            && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                token_idx as usize + group_len,
                tokens,
                ignore_span,
                enclosing_span,
            )
            && let Some(forward_target_id) = find_node_starting_at(
                source_map,
                &forward_token.span,
                NodeSearchMode::SmallestOutermost,
            )
            .or_else(|| {
                find_node_enclosing_at(
                    source_map,
                    &forward_token.span,
                    NodeSearchMode::SmallestOutermost,
                    |candidate| !is_annotation_node_id(tree, candidate.idx),
                )
            })
            .map(|span| span.idx)
        {
            if prev_token.token.ty == TokenType::Colon
                && let Some(mapped_value_target_id) = mapped_type_value_target_for_separator(
                    tree,
                    source_map,
                    prev_token,
                    forward_token,
                    forward_target_id,
                    ignore_span,
                )
            {
                return Some((
                    AnnotationPosition::LinePostfixBoundary,
                    mapped_value_target_id,
                ));
            }

            return Some((
                AnnotationPosition::LinePrefix,
                annotation_promote_statement(
                    tree,
                    start_token,
                    forward_target_id,
                    statement_wrappers,
                ),
            ));
        }
        // comments right after `implements` belong on the extends seam line, before `implements`
        else if start_token.token.ty == TokenType::LineComment
            && let previous_keyword_token = prev_token.and_then(|(prev_idx, prev_token)| {
                if prev_token.token.ty == TokenType::Newline {
                    annotation_prev_significant_token(prev_idx, tokens, ignore_span, enclosing_span)
                } else {
                    Some(prev_token)
                }
            })
            && let Some(prev_token) = previous_keyword_token
            && annotation_is_identifier_keyword(file, prev_token, "implements")
            && let Some(declaration_owner_id) = find_node_enclosing_at(
                source_map,
                &prev_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| {
                    !is_annotation_node_id(tree, candidate.idx)
                        && tree.get_node_type(candidate.idx) == NodeType::Declaration
                },
            )
            .map(|span| span.idx)
            && let Some(extends_target_id) =
                declaration_last_extends_expression(tree, declaration_owner_id)
        {
            return Some((AnnotationPosition::LinePostfixBoundary, extends_target_id));
        }
        // comments between declaration names and `<...>` generic heads stay on the declaration seam
        else if start_token.token.ty == TokenType::LineComment
            && next_token.is_some_and(|(_, token)| token.token.ty == TokenType::Newline)
            && let Some(previous_significant_token) = annotation_prev_significant_token(
                token_idx as usize,
                tokens,
                ignore_span,
                enclosing_span,
            )
            && previous_significant_token.token.ty == TokenType::Identifier
            && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                token_idx as usize + group_len,
                tokens,
                ignore_span,
                enclosing_span,
            )
            && forward_token.token.ty == TokenType::LessThan
            && let Some(declaration_owner_id) = find_node_enclosing_at(
                source_map,
                &forward_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| {
                    !is_annotation_node_id(tree, candidate.idx)
                        && tree.get_node_type(candidate.idx) == NodeType::Declaration
                },
            )
            .map(|span| span.idx)
        {
            return Some((AnnotationPosition::LinePrefix, declaration_owner_id));
        }
        // comments before a leading semicolon stay on the previous statement boundary
        else if start_token.token.ty == TokenType::LineComment
            && let Some((_, prev_token)) = prev_token
            && let Some((_, forward_token)) = annotation_next_non_annotation_token(
                token_idx as usize + group_len,
                tokens,
                ignore_span,
                None,
            )
            && forward_token.token.ty == TokenType::Semicolon
            && let Some(statement_owner_id) = find_node_enclosing_at(
                source_map,
                &prev_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| {
                    !is_annotation_node_id(tree, candidate.idx)
                        && tree.get_node_type(candidate.idx) == NodeType::Expression
                        && matches!(
                            tree.get(LocalNodeId::<Expression>::new(candidate.idx)),
                            Expression::Statement(_)
                        )
                },
            )
            .map(|span| span.idx)
        {
            return Some((AnnotationPosition::LinePostfixBoundary, statement_owner_id));
        }
        // comments after statement semicolons stay on that statement line
        else if let Some((prev_idx, prev_token)) = prev_token
            && prev_token.token.ty == TokenType::Semicolon
            && line_indices[prev_idx] == line_indices[group_end_idx]
            && (next_token.is_none()
                || next_token.unwrap().1.token.ty == TokenType::Newline
                || next_token.unwrap().1.token.ty == TokenType::End)
            && let Some(statement_owner_id) = find_node_enclosing_at(
                source_map,
                &prev_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| {
                    !is_annotation_node_id(tree, candidate.idx)
                        && tree.get_node_type(candidate.idx) == NodeType::Expression
                },
            )
            .map(|span| span.idx)
        {
            return Some((AnnotationPosition::LinePostfixBoundary, statement_owner_id));
        }
        // directive-tail block comments like `"use strict" /**/` stay on the directive line
        else if is_block_comment_single_line
            && let Some((prev_idx, prev_token)) = prev_token
            && line_indices[prev_idx] == line_indices[group_end_idx]
            && (next_token.is_none()
                || next_token.unwrap().1.token.ty == TokenType::Newline
                || next_token.unwrap().1.token.ty == TokenType::End)
            && let Some(statement_owner_id) = find_node_enclosing_at(
                source_map,
                &prev_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| {
                    !is_annotation_node_id(tree, candidate.idx)
                        && tree.get_node_type(candidate.idx) == NodeType::Expression
                },
            )
            .map(|span| span.idx)
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(statement_owner_id)),
                Expression::Statement(_)
            )
        {
            return Some((AnnotationPosition::LinePostfixBoundary, statement_owner_id));
        }
        // inline block comment before a node: keep on the same line
        else if start_token.token.ty == TokenType::LineComment
            && let Some((_, prev_token)) = prev_token
            && prev_token.token.ty == TokenType::ElementwiseAnd
            && next_token.is_some_and(|(_, token)| token.token.ty == TokenType::Newline)
        {
            // keep `A & // note \n B` attached to the left member
            let mut separator_owner = None;
            let mut before_separator_idx = token_idx as usize;
            while before_separator_idx > 0 {
                before_separator_idx -= 1;
                let Some(before_token) = tokens.get(before_separator_idx).copied() else {
                    break;
                };
                if ignore_span.contains(&before_token.span)
                    || before_token.token.ty == TokenType::Whitespace
                    || before_token.token.ty == TokenType::Newline
                {
                    continue;
                }
                if before_token.token.ty == TokenType::ElementwiseAnd {
                    continue;
                }

                separator_owner = find_node_ending_at(
                    source_map,
                    &before_token.span,
                    NodeSearchMode::SmallestOutermost,
                )
                .map(|span| span.idx);
                break;
            }

            if let Some(target_node_id) = separator_owner {
                return Some((AnnotationPosition::LinePostfixBoundary, target_node_id));
            }
        }
        // inline block comment before a node: keep on the same line
        else if is_block_comment_single_line
            && let Some((next_idx, next_token)) = next_token
            && next_token.token.ty != TokenType::Newline
            && !matches!(
                next_token.token.ty,
                TokenType::CloseParenthesis
                    | TokenType::CloseBrace
                    | TokenType::CloseBracket
                    | TokenType::End
            )
            && (line_indices[group_end_idx] == line_indices[next_idx]
                || line_indices[group_start_idx] == line_indices[next_idx])
        {
            let target_node_id = find_node_starting_at(
                source_map,
                &next_token.span,
                NodeSearchMode::SmallestOutermost,
            )
            .or_else(|| {
                find_node_enclosing_at(
                    source_map,
                    &next_token.span,
                    NodeSearchMode::SmallestOutermost,
                    |candidate| {
                        !is_annotation_node_id(tree, candidate.idx)
                            && !ignore_span.contains(&candidate.span)
                    },
                )
            })
            .or_else(|| {
                find_node_enclosing_at(
                    source_map,
                    &next_token.span,
                    NodeSearchMode::BiggestOutermost,
                    |candidate| {
                        !is_annotation_node_id(tree, candidate.idx)
                            && !ignore_span.contains(&candidate.span)
                    },
                )
            })
            .map(|span| span.idx);

            if let Some(target_node_id) = target_node_id {
                let target_node_id = annotation_argument_value_expression_id(tree, target_node_id)
                    .unwrap_or(target_node_id);
                return Some((
                    AnnotationPosition::LinePrefix,
                    annotation_promote_statement(
                        tree,
                        start_token,
                        target_node_id,
                        statement_wrappers,
                    ),
                ));
            }
        }
        // special case: inline comment between path segments attaches to the path as postfix
        else if let Some((_, prev_token)) = prev_token
            && let Some(enclosing_scope) = enclosing_scope
            && tree.get_node_type(enclosing_scope.idx) == NodeType::Expression
        {
            let expression_id = LocalNodeId::<Expression>::new(enclosing_scope.idx);
            if let Expression::Path { path, .. } = tree.get(expression_id)
                && path.segments.len() > 1
                && enclosing_scope.span.end > prev_token.span.end
            {
                return Some((
                    AnnotationPosition::LinePostfixBoundary,
                    annotation_promote_statement(
                        tree,
                        start_token,
                        expression_id.id,
                        statement_wrappers,
                    ),
                ));
            }
        }
        // line prefix: check for directly following node that starts at the end token
        else if let Some((next_idx, next_token)) = next_token
            && next_token.token.ty != TokenType::Newline
            && !matches!(
                next_token.token.ty,
                TokenType::CloseParenthesis
                    | TokenType::CloseBrace
                    | TokenType::CloseBracket
                    | TokenType::End
            )
            && (line_indices[group_end_idx] == line_indices[next_idx]
                || line_indices[group_start_idx] == line_indices[next_idx])
        {
            let target_node_id = find_node_starting_at(
                source_map,
                &next_token.span,
                NodeSearchMode::SmallestOutermost,
            )
            .or_else(|| {
                find_node_enclosing_at(
                    source_map,
                    &next_token.span,
                    NodeSearchMode::SmallestOutermost,
                    |candidate| {
                        !is_annotation_node_id(tree, candidate.idx)
                            && !ignore_span.contains(&candidate.span)
                    },
                )
            })
            .map(|span| span.idx);

            if let Some(target_node_id) = target_node_id {
                return Some((
                    AnnotationPosition::LinePrefix,
                    annotation_promote_statement(
                        tree,
                        start_token,
                        target_node_id,
                        statement_wrappers,
                    ),
                ));
            }
        }
        // comments before interpolation close braces should stay on the preceding expression boundary
        else if start_token.token.ty == TokenType::LineComment
            && next_token.is_some_and(|(_, token)| token.token.ty == TokenType::Newline)
            && annotation_next_non_annotation_token(
                token_idx as usize + group_len,
                tokens,
                ignore_span,
                enclosing_span,
            )
            .is_some_and(|(_, token)| token.token.ty == TokenType::CloseBrace)
            && let Some((_, prev_token)) = prev_token
            && let Some(target_node_id) = find_node_ending_at(
                source_map,
                &prev_token.span,
                NodeSearchMode::BiggestOutermost,
            )
            .map(|span| span.idx)
        {
            return Some((
                AnnotationPosition::LinePostfixBoundary,
                annotation_promote_statement(tree, start_token, target_node_id, statement_wrappers),
            ));
        }
    }

    // block prefix: find the following targetable node
    let mut next_token_idx = token_idx as usize + group_len;
    while let Some(next_token) = tokens.get(next_token_idx) {
        if ignore_span.contains(&next_token.span)
            || ANNOTATION_TOKEN_TYPES.contains(&next_token.token.ty)
            || next_token.token.ty == TokenType::Whitespace
        {
            next_token_idx += 1;
            continue;
        }
        if let Some(enclosing_span) = enclosing_span
            && !enclosing_span.intersects(next_token.span)
            && start_token.token.ty != TokenType::Newline
        {
            break;
        }
        let next_node = if is_block_prefix_only {
            find_node_starting_at(
                source_map,
                &next_token.span,
                NodeSearchMode::BiggestOutermost,
            )
            .or_else(|| {
                find_node_enclosing_at(
                    source_map,
                    &next_token.span,
                    NodeSearchMode::SmallestOutermost,
                    |candidate| {
                        !is_annotation_node_id(tree, candidate.idx)
                            && !ignore_span.contains(&candidate.span)
                    },
                )
            })
        } else {
            find_node_starting_at(
                source_map,
                &next_token.span,
                NodeSearchMode::BiggestOutermost,
            )
        };

        if let Some(next_node) = next_node {
            let target_node_id =
                annotation_promote_statement(tree, start_token, next_node.idx, statement_wrappers);
            return Some((AnnotationPosition::BlockPrefix, target_node_id));
        }
        if next_token.token.ty == TokenType::Dot {
            let target_node_id = find_node_enclosing_at(
                source_map,
                &next_token.span,
                NodeSearchMode::SmallestOutermost,
                |candidate| {
                    !is_annotation_node_id(tree, candidate.idx)
                        && !ignore_span.contains(&candidate.span)
                },
            )
            .map(|span| span.idx);
            if let Some(target_node_id) = target_node_id {
                return Some((
                    AnnotationPosition::BlockPrefix,
                    annotation_promote_statement(
                        tree,
                        start_token,
                        target_node_id,
                        statement_wrappers,
                    ),
                ));
            }
        }
        next_token_idx += 1;
    }

    // block postfix: find the preceding targetable node
    if !is_block_prefix_only && token_idx > 0 {
        let mut prev_token_idx = token_idx as usize;
        while prev_token_idx > 0 {
            prev_token_idx -= 1;
            let Some(prev_token) = tokens.get(prev_token_idx) else {
                break;
            };
            if ignore_span.contains(&prev_token.span)
                || ANNOTATION_TOKEN_TYPES.contains(&prev_token.token.ty)
                || prev_token.token.ty == TokenType::Whitespace
            {
                continue;
            }
            if let Some(enclosing_span) = enclosing_span
                && !enclosing_span.intersects(prev_token.span)
            {
                break;
            }
            if let Some(prev_node) = find_node_ending_at(
                source_map,
                &prev_token.span,
                NodeSearchMode::BiggestOutermost,
            ) {
                return Some((
                    AnnotationPosition::BlockPostfix,
                    annotation_promote_statement(
                        tree,
                        start_token,
                        prev_node.idx,
                        statement_wrappers,
                    ),
                ));
            }
        }
    }

    // find inner enclosing node (block infix)
    if !is_block_prefix_only && let Some(enclosing_node) = enclosing_scope {
        return Some((
            AnnotationPosition::BlockInfix,
            annotation_promote_statement(tree, start_token, enclosing_node.idx, statement_wrappers),
        ));
    }

    None
}

/// Collect semantic and side tokens without whitespace in source order.
fn collect_annotation_tokens(
    tokens: &[TokenSpan],
    side_tokens: &[TokenSpan],
    file: &File,
) -> (Vec<TokenSpan>, Vec<u32>) {
    let mut merged = Vec::with_capacity(tokens.len() + side_tokens.len());
    let mut line_indices = Vec::with_capacity(tokens.len() + side_tokens.len());

    let mut main_index = 0usize;
    let mut side_index = 0usize;
    loop {
        while let Some(token) = tokens.get(main_index) {
            if token.token.ty != TokenType::Whitespace {
                break;
            }
            main_index += 1;
        }

        while let Some(token) = side_tokens.get(side_index) {
            if token.token.ty != TokenType::Whitespace {
                break;
            }
            side_index += 1;
        }

        let main_token = tokens.get(main_index).copied();
        let side_token = side_tokens.get(side_index).copied();
        let next = match (main_token, side_token) {
            (Some(main_token), Some(side_token)) => {
                if main_token.span.start <= side_token.span.start {
                    main_index += 1;
                    Some(main_token)
                } else {
                    side_index += 1;
                    Some(side_token)
                }
            }
            (Some(main_token), None) => {
                main_index += 1;
                Some(main_token)
            }
            (None, Some(side_token)) => {
                side_index += 1;
                Some(side_token)
            }
            (None, None) => None,
        };

        let Some(next) = next else {
            break;
        };

        let line_index = file
            .get_position(next.span.start)
            .map_or(0, |position| position.0);
        merged.push(next);
        line_indices.push(line_index);
    }

    (merged, line_indices)
}

/// Build formatter-owned annotation ids and entries from semantic attachments and trivia.
fn build_formatter_annotation_projection(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    side_tokens: &[TokenSpan],
    side_span: &MultiSpan,
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
                AstAnnotation::Doc { node, position } => Annotation::Doc {
                    node: *node,
                    position: *position,
                },
                AstAnnotation::Decorator { node, position } => Annotation::Decorator {
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

    // side annotation mapping
    let statement_wrappers = collect_statement_wrappers(tree);
    let (annotation_tokens, line_indices) = collect_annotation_tokens(tokens, side_tokens, file);

    let mut token_index_by_span_and_type = FxHashMap::<(u32, u32, TokenType), usize>::default();
    let mut newline_index_by_start = FxHashMap::<u32, usize>::default();
    for (token_index, token) in annotation_tokens.iter().copied().enumerate() {
        if !is_side_annotation_token(token.token.ty) {
            continue;
        }
        token_index_by_span_and_type.insert(
            (token.span.start, token.span.end, token.token.ty),
            token_index,
        );
        if token.token.ty == TokenType::Newline {
            newline_index_by_start.insert(token.span.start, token_index);
        }
    }

    // add formatter-placed comment trivia with legacy-compatible targeting
    let debug_trivia = std::env::var("DESTACK_DEBUG_TRIVIA").is_ok();
    for trivia in tree.comment_trivia().iter().copied() {
        let comment = tree.get::<Comment>(trivia.comment);
        let token_index = match comment.style {
            CommentStyle::Slash => token_index_by_span_and_type
                .get(&(trivia.span.start, trivia.span.end, TokenType::LineComment))
                .copied()
                .or_else(|| {
                    token_index_by_span_and_type
                        .get(&(
                            trivia.span.start,
                            trivia.span.end,
                            TokenType::DocLineComment,
                        ))
                        .copied()
                }),
            CommentStyle::Star => token_index_by_span_and_type
                .get(&(trivia.span.start, trivia.span.end, TokenType::BlockComment))
                .copied()
                .or_else(|| {
                    token_index_by_span_and_type
                        .get(&(
                            trivia.span.start,
                            trivia.span.end,
                            TokenType::DocBlockComment,
                        ))
                        .copied()
                }),
        };
        let Some(token_index) = token_index else {
            continue;
        };

        let Some((position, target_id)) = find_side_annotation_target(
            tree,
            &tree.source_map,
            file,
            token_index as u32,
            &annotation_tokens,
            &line_indices,
            token_index,
            token_index,
            1,
            comment.style == CommentStyle::Slash,
            false,
            &statement_wrappers,
            side_span,
        ) else {
            continue;
        };
        if debug_trivia {
            let comment_text = file.span_str(trivia.span);
            let previous_non_whitespace = file
                .span_str(Span::new(file.id, 0, trivia.span.start))
                .chars()
                .rev()
                .find(|character| !character.is_whitespace());
            let target_type = tree.get_node_type(target_id);
            if target_type == NodeType::Expression {
                let expression = tree.get(LocalNodeId::<Expression>::new(target_id));
                eprintln!(
                    "comment {:?} => {:?} on {} ({:?}) prev_char={:?} expression={:?}",
                    comment_text,
                    position,
                    target_id,
                    target_type,
                    previous_non_whitespace,
                    expression
                );
            } else {
                eprintln!(
                    "comment {:?} => {:?} on {} ({:?}) prev_char={:?}",
                    comment_text, position, target_id, target_type, previous_non_whitespace
                );
            }
        }
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

    // add formatter-placed blank trivia with legacy-compatible targeting
    for trivia in tree.blank_trivia().iter().copied() {
        let Some(start_token_index) = newline_index_by_start.get(&trivia.span.start).copied()
        else {
            continue;
        };

        let mut end_token_index = start_token_index;
        let mut cursor = start_token_index;
        while let Some(token) = annotation_tokens.get(cursor).copied() {
            if token.token.ty != TokenType::Newline {
                break;
            }
            if token.span.start < trivia.span.start || token.span.end > trivia.span.end {
                break;
            }
            end_token_index = cursor;
            cursor += 1;
        }

        let group_len = end_token_index.saturating_sub(start_token_index) + 1;
        if group_len <= 1 {
            continue;
        }

        let Some((position, target_id)) = find_side_annotation_target(
            tree,
            &tree.source_map,
            file,
            start_token_index as u32,
            &annotation_tokens,
            &line_indices,
            start_token_index,
            end_token_index,
            group_len,
            false,
            false,
            &statement_wrappers,
            side_span,
        ) else {
            continue;
        };
        if debug_trivia {
            if tree.get_node_type(target_id) == NodeType::Expression {
                eprintln!(
                    "blank {:?} => {:?} on {} ({:?}) expression={:?}",
                    file.span_str(trivia.span),
                    position,
                    target_id,
                    tree.get_node_type(target_id),
                    tree.get(LocalNodeId::<Expression>::new(target_id))
                );
            } else {
                eprintln!(
                    "blank {:?} => {:?} on {} ({:?})",
                    file.span_str(trivia.span),
                    position,
                    target_id,
                    tree.get_node_type(target_id)
                );
            }
        }
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
    call_argument_expansion_profiles: RefCell<Vec<Option<CachedCallArgumentExpansionProfiles>>>,
    /// Cached inline call length estimates without static arguments keyed by call expression id.
    call_inline_len_without_static_arguments: RefCell<Vec<Option<Option<usize>>>>,
    /// Cached call argument annotation profiles keyed by argument node id.
    argument_annotation_profile: RefCell<Vec<Option<CachedArgumentAnnotationProfile>>>,
    /// Cached compact simple unannotated argument predicate keyed by argument node id.
    argument_compact_simple_unannotated: RefCell<Vec<Option<bool>>>,
    /// Cached plain-call-argument predicate keyed by argument node id.
    argument_plain_call_argument: RefCell<Vec<Option<bool>>>,
    /// Cached call argument layout-class facts keyed by call expression node id.
    call_argument_layout_class: RefCell<Vec<Option<CachedCallArgumentLayoutClass>>>,
    /// Cached chain call force-expand decisions keyed by call expression node id.
    call_argument_chain_force_expand: RefCell<Vec<Option<bool>>>,
    /// Cached transparent inner expression ids keyed by expression node id.
    transparent_inner_expression: RefCell<Vec<Option<LocalNodeId<Expression>>>>,
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
            call_argument_expansion_profiles: RefCell::new(vec![None; node_count]),
            call_inline_len_without_static_arguments: RefCell::new(vec![None; node_count]),
            argument_annotation_profile: RefCell::new(vec![None; node_count]),
            argument_compact_simple_unannotated: RefCell::new(vec![None; node_count]),
            argument_plain_call_argument: RefCell::new(vec![None; node_count]),
            call_argument_layout_class: RefCell::new(vec![None; node_count]),
            call_argument_chain_force_expand: RefCell::new(vec![None; node_count]),
            transparent_inner_expression: RefCell::new(vec![None; node_count]),
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
            build_formatter_annotation_projection(file, tree, tokens, side_tokens, side_span);
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

        {
            let cache = self.node_caches.transparent_inner_expression.borrow();
            if let Some(inner_expression_id) = cache[node_index] {
                return inner_expression_id;
            }
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

        let mut cache = self.node_caches.transparent_inner_expression.borrow_mut();
        for expression_index in visited_expression_indices {
            cache[expression_index] = Some(current_id);
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
                            if comment.style != destack_ast::CommentStyle::Slash {
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
    fn cache_get_copy_entry<T: Copy>(
        &self,
        cache: &RefCell<Vec<Option<T>>>,
        node_id: u32,
    ) -> Option<T> {
        cache
            .borrow()
            .get(node_id as usize)
            .and_then(|entry| entry.as_ref())
            .copied()
    }

    /// Write one copyable value into an index-addressed optional cache.
    fn cache_set_copy_entry<T: Copy>(
        &self,
        cache: &RefCell<Vec<Option<T>>>,
        node_id: u32,
        value: T,
    ) {
        let mut cache = cache.borrow_mut();
        if node_id as usize >= cache.len() {
            cache.resize((node_id + 1) as usize, None);
        }
        cache[node_id as usize] = Some(value);
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
