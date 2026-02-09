use std::cell::{Cell, OnceCell, Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use destack_ast::{
    Annotation, AnnotationPosition, Argument, Blank, Block, Comment, Declaration, Declarator,
    Decorator, DependencyItem, Doc, EnumField, Expression, LocalNodeId, LocalNodeIdAny, MatchCase,
    Member, Node, NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern,
    PatternField, Property, TokenSpan, TokenType, WhereClause,
};
use destack_base::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter, GroupId};
use destack_fir::print::PrintOptions;
use destack_source::{File, IndentStyle, LanguageType, LineEnding, MultiSpan, NodeSourceMap, Span};
use destack_workspace::{
    ArrowParentheses, FormatterOptions, ImportSortOrder, OrganizeImports, QuoteProperty,
    QuoteStyle, TrailingComma,
};

use super::timing::{
    FormatterTimingEntry, FormatterTimingScope, FormatterTimingTag, FormatterTimings,
    tag_for_node_type, timings_enabled_from_env,
};

const ANNOTATION_STATE_UNKNOWN: u8 = 0;
const ANNOTATION_STATE_NONE: u8 = 1;
const ANNOTATION_STATE_PRESENT: u8 = 2;
const ANNOTATION_STATE_CACHED: u8 = 3;

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
    pub counters: RefCell<HashMap<&'static str, usize>>,
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
                name: *name,
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
    /// Whether any annotation is a prefix annotation.
    pub has_prefix: bool,
    /// Whether any annotation is an infix annotation.
    pub has_infix: bool,
    /// Whether any annotation is a postfix annotation.
    pub has_postfix: bool,
    /// Whether any annotation is a blank prefix annotation.
    pub has_blank_prefix: bool,
    /// Whether the first annotation is a blank prefix annotation.
    pub has_blank_prefix_first: bool,
}

impl CachedAnnotationData {
    /// Build cached annotation metadata from annotation ids.
    pub fn from_ids(tree: &NodeTree, ids: Vec<LocalNodeId<Annotation>>) -> Self {
        let mut has_prefix = false;
        let mut has_infix = false;
        let mut has_postfix = false;
        let mut has_blank_prefix = false;
        let mut has_blank_prefix_first = false;

        for (index, annotation_id) in ids.iter().enumerate() {
            let annotation = tree.get::<Annotation>(*annotation_id);
            let position = annotation.position();

            // general position flags
            if position == AnnotationPosition::BlockPrefix
                || position == AnnotationPosition::LinePrefix
            {
                has_prefix = true;
            }

            if position == AnnotationPosition::BlockInfix {
                has_infix = true;
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
            has_prefix,
            has_infix,
            has_postfix,
            has_blank_prefix,
            has_blank_prefix_first,
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
    /// Cached node-to-annotation ids and annotation metadata for hot annotation lookups.
    pub annotation_ids_cache: RefCell<Vec<Option<CachedAnnotationData>>>,
    /// Cached annotation presence for node ids.
    pub annotation_presence_cache: RefCell<Vec<u8>>,
    /// Cached source slices for repeated span lookups.
    pub span_text_cache: RefCell<HashMap<Span, &'a str>>,
    /// Cached char lengths for repeated span width checks.
    pub span_char_len_cache: RefCell<HashMap<Span, usize>>,
    /// Cached newline checks for repeated span newline predicates.
    pub span_has_newline_cache: RefCell<HashMap<Span, bool>>,
    /// Cached comment checks for repeated span comment predicates.
    pub span_has_comment_cache: RefCell<HashMap<Span, bool>>,
    /// Cached call argument expand decisions for chain planning keyed by call node id.
    pub call_chain_argument_expand_cache: RefCell<HashMap<u32, bool>>,
    /// Cached sorted comment tokens for ignore-range and comment-boundary scans.
    pub comment_tokens_cache: OnceCell<Vec<TokenSpan>>,
    /// Comment spans for this file, sorted by start position.
    pub comment_spans: Vec<Span>,
    /// Optional formatter timing collector.
    pub timings: Option<Rc<FormatterTimings>>,
    /// Whether instrumentation counters should be collected.
    pub instrumentation_enabled: bool,
    /// Shared cache instrumentation counters.
    pub cache_stats: Rc<FormatterCacheStatsCollector>,
    /// Shared generic instrumentation counters.
    pub counters: Rc<FormatterCountersCollector>,
}

impl<'a> DestackFormatContext<'a> {
    /// Construct a formatting context from parse artifacts.
    pub fn new(
        options: DestackFormatOptions,
        file: &'a File,
        tree: &'a NodeTree,
        tokens: &'a Vec<TokenSpan>,
        side_tokens: &'a Vec<TokenSpan>,
        side_span: &'a MultiSpan,
        strings: &'a ImmutableStringPool,
        parents: NodeParentIndex,
    ) -> Self {
        Self::new_with_timings(
            options,
            file,
            tree,
            tokens,
            side_tokens,
            side_span,
            strings,
            parents,
            false,
        )
    }

    /// Construct a formatting context from parse artifacts with optional timing collection.
    pub fn new_with_timings(
        options: DestackFormatOptions,
        file: &'a File,
        tree: &'a NodeTree,
        tokens: &'a Vec<TokenSpan>,
        side_tokens: &'a Vec<TokenSpan>,
        side_span: &'a MultiSpan,
        strings: &'a ImmutableStringPool,
        parents: NodeParentIndex,
        timings_enabled: bool,
    ) -> Self {
        let timings_enabled = timings_enabled || timings_enabled_from_env();
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
            annotation_ids_cache: RefCell::new(vec![None; tree.next_id() as usize]),
            annotation_presence_cache: RefCell::new(vec![
                ANNOTATION_STATE_UNKNOWN;
                tree.next_id() as usize
            ]),
            span_text_cache: RefCell::new(HashMap::new()),
            span_char_len_cache: RefCell::new(HashMap::new()),
            span_has_newline_cache: RefCell::new(HashMap::new()),
            span_has_comment_cache: RefCell::new(HashMap::new()),
            call_chain_argument_expand_cache: RefCell::new(HashMap::new()),
            comment_tokens_cache: OnceCell::new(),
            comment_spans,
            timings: timings_enabled.then(|| Rc::new(FormatterTimings::default())),
            instrumentation_enabled: timings_enabled,
            cache_stats: Rc::new(FormatterCacheStatsCollector::default()),
            counters: Rc::new(FormatterCountersCollector::default()),
        }
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

        let len = self.get_span_str(span).chars().count();
        self.span_char_len_cache.borrow_mut().insert(span, len);
        len
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
        let has_newline = self.get_span_str(span).contains('\n');
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
        {
            let presence = self.annotation_presence_cache.borrow();
            let state = presence
                .get(node_id.id as usize)
                .copied()
                .unwrap_or(ANNOTATION_STATE_UNKNOWN);
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
                drop(presence);
                let cache = self.annotation_ids_cache.borrow();
                return Some(Ref::map(cache, |cache| {
                    cache
                        .get(node_id.id as usize)
                        .and_then(|entry| entry.as_ref())
                        .expect("annotation cache should contain requested node")
                }));
            }
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .annotation_cache_misses
                .set(self.cache_stats.annotation_cache_misses.get() + 1);
        }

        // lookup annotations once and cache both presence and metadata
        let node_has_annotations = if self
            .annotation_presence_cache
            .borrow()
            .get(node_id.id as usize)
            .copied()
            .unwrap_or(ANNOTATION_STATE_UNKNOWN)
            == ANNOTATION_STATE_PRESENT
        {
            true
        } else {
            self.tree.has_annotations(node_id.id)
        };
        if !node_has_annotations {
            let mut presence = self.annotation_presence_cache.borrow_mut();
            if node_id.id as usize >= presence.len() {
                presence.resize((node_id.id + 1) as usize, ANNOTATION_STATE_UNKNOWN);
            }
            presence[node_id.id as usize] = ANNOTATION_STATE_NONE;
            return None;
        }

        let annotation_data =
            CachedAnnotationData::from_ids(self.tree, self.tree.get_annotations(node_id.id));
        {
            let mut cache = self.annotation_ids_cache.borrow_mut();
            if node_id.id as usize >= cache.len() {
                cache.resize((node_id.id + 1) as usize, None);
            }
            cache[node_id.id as usize] = Some(annotation_data);
        }
        {
            let mut presence = self.annotation_presence_cache.borrow_mut();
            if node_id.id as usize >= presence.len() {
                presence.resize((node_id.id + 1) as usize, ANNOTATION_STATE_UNKNOWN);
            }
            presence[node_id.id as usize] = ANNOTATION_STATE_CACHED;
        }

        let cache = self.annotation_ids_cache.borrow();
        Some(Ref::map(cache, |cache| {
            cache
                .get(node_id.id as usize)
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
        let state = self
            .annotation_presence_cache
            .borrow()
            .get(node_id.id as usize)
            .copied()
            .unwrap_or(ANNOTATION_STATE_UNKNOWN);
        if state == ANNOTATION_STATE_NONE {
            if self.instrumentation_enabled {
                self.cache_stats
                    .annotation_cache_hits
                    .set(self.cache_stats.annotation_cache_hits.get() + 1);
            }
            return false;
        }
        if state == ANNOTATION_STATE_PRESENT || state == ANNOTATION_STATE_CACHED {
            if self.instrumentation_enabled {
                self.cache_stats
                    .annotation_cache_hits
                    .set(self.cache_stats.annotation_cache_hits.get() + 1);
            }
            return true;
        }

        if self.instrumentation_enabled {
            self.cache_stats
                .annotation_cache_misses
                .set(self.cache_stats.annotation_cache_misses.get() + 1);
        }

        let has_annotation = self.tree.has_annotations(node_id.id);
        let mut presence = self.annotation_presence_cache.borrow_mut();
        if node_id.id as usize >= presence.len() {
            presence.resize((node_id.id + 1) as usize, ANNOTATION_STATE_UNKNOWN);
        }
        presence[node_id.id as usize] = if has_annotation {
            ANNOTATION_STATE_PRESENT
        } else {
            ANNOTATION_STATE_NONE
        };
        has_annotation
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

    /// Start a formatter timing scope.
    #[inline]
    pub fn timing_scope(&self, tag: FormatterTimingTag) -> FormatterTimingScope {
        FormatterTimingScope::new(self.timings.clone(), tag)
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
                let node = context.tree.get(node_id);
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
