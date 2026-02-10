use std::cell::{Cell, OnceCell, Ref, RefCell};
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
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use super::timing::{
    FormatterTimingEntry, FormatterTimingScope, FormatterTimingTag, FormatterTimings,
    tag_for_node_type, timings_enabled_from_env,
};

const ANNOTATION_STATE_UNKNOWN: u8 = 0;
const ANNOTATION_STATE_NONE: u8 = 1;
const ANNOTATION_STATE_PRESENT: u8 = 2;
const ANNOTATION_STATE_CACHED: u8 = 3;
const TYPE_CONTEXT_STATE_UNKNOWN: u8 = 0;
const TYPE_CONTEXT_STATE_FALSE: u8 = 1;
const TYPE_CONTEXT_STATE_TRUE: u8 = 2;

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
    pub fn from_ids(tree: &NodeTree, ids: Vec<LocalNodeId<Annotation>>) -> Self {
        let mut has_non_blank = false;
        let mut has_prefix = false;
        let mut has_infix = false;
        let mut has_non_blank_infix = false;
        let mut has_postfix = false;
        let mut has_blank_prefix = false;
        let mut has_blank_prefix_first = false;

        for (index, annotation_id) in ids.iter().enumerate() {
            let annotation = tree.get::<Annotation>(*annotation_id);
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

/// Cached argument annotation facts used by hot call formatting paths.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachedArgumentAnnotationProfile {
    /// Whether the argument has any comment annotation.
    pub has_comment: bool,
    /// Whether the argument has a trailing slash style comment annotation.
    pub has_line_comment: bool,
    /// Whether the argument has a slash style prefix comment annotation.
    pub has_prefix_line_comment: bool,
}

/// Cached call argument expansion facts keyed by call expression node id.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachedCallArgumentFacts {
    /// Whether any argument has a line comment annotation.
    pub has_line_comment_annotations: bool,
    /// Whether the last argument is a collection literal.
    pub trailing_collection_argument: bool,
    /// Whether any argument is a block callback.
    pub has_block_callback_argument: bool,
    /// Whether the first argument is a block callback.
    pub first_argument_is_block_callback: bool,
    /// Whether the last argument is a block callback.
    pub last_argument_is_block_callback: bool,
    /// Whether any non-last, non-callback argument is non-trivial.
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
    /// Cached node-to-annotation ids and annotation metadata for hot annotation lookups.
    pub annotation_ids_cache: RefCell<Vec<Option<CachedAnnotationData>>>,
    /// Cached annotation presence for node ids.
    pub annotation_presence_cache: Vec<Cell<u8>>,
    /// Cached source slices for repeated span lookups.
    pub span_text_cache: RefCell<FxHashMap<Span, &'a str>>,
    /// Cached char lengths for repeated span width checks.
    pub span_char_len_cache: RefCell<FxHashMap<Span, usize>>,
    /// Cached newline checks for repeated span newline predicates.
    pub span_has_newline_cache: RefCell<FxHashMap<Span, bool>>,
    /// Cached comment checks for repeated span comment predicates.
    pub span_has_comment_cache: RefCell<FxHashMap<Span, bool>>,
    /// Cached span char lengths for node ids.
    pub node_span_char_len_cache: RefCell<Vec<Option<usize>>>,
    /// Cached node span newline predicates keyed by node id.
    pub node_has_newline_cache: RefCell<Vec<Option<bool>>>,
    /// Cached call argument expansion profiles for regular and chain modes keyed by call node id.
    pub call_argument_expansion_profiles_cache:
        RefCell<Vec<Option<CachedCallArgumentExpansionProfiles>>>,
    /// Cached call argument annotation profiles keyed by argument node id.
    pub argument_annotation_profile_cache: RefCell<Vec<Option<CachedArgumentAnnotationProfile>>>,
    /// Cached call argument expansion facts keyed by call expression node id.
    pub call_argument_facts_cache: RefCell<Vec<Option<CachedCallArgumentFacts>>>,
    /// Cached chain call force-expand decisions keyed by call expression node id.
    pub call_argument_chain_force_expand_cache: RefCell<Vec<Option<bool>>>,
    /// Cached transparent inner expression ids keyed by expression node id.
    pub transparent_inner_expression_cache: RefCell<Vec<Option<LocalNodeId<Expression>>>>,
    /// Cached type-context decisions keyed by expression node id.
    pub expression_type_context_cache: Vec<Cell<u8>>,
    /// Cached template interpolation ancestry decisions keyed by expression node id.
    pub expression_template_interpolation_cache: Vec<Cell<u8>>,
    /// Cached type-conditional ancestry decisions keyed by expression node id.
    pub expression_type_conditional_ancestor_cache: Vec<Cell<u8>>,
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
        let timings_enabled = timings_enabled || timings_enabled_from_env();
        let file_text = file.text();
        let has_ignore_directive_markers = file_text.contains("format-ignore")
            || file_text.contains("fmt-ignore")
            || file_text.contains("deno-fmt-ignore")
            || file_text.contains("prettier-ignore")
            || file_text.contains("biome-ignore format")
            || file_text.contains("oxfmt-ignore");
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
            annotation_presence_cache: vec![
                Cell::new(ANNOTATION_STATE_UNKNOWN);
                tree.next_id() as usize
            ],
            span_text_cache: RefCell::new(FxHashMap::default()),
            span_char_len_cache: RefCell::new(FxHashMap::default()),
            span_has_newline_cache: RefCell::new(FxHashMap::default()),
            span_has_comment_cache: RefCell::new(FxHashMap::default()),
            node_span_char_len_cache: RefCell::new(vec![None; tree.next_id() as usize]),
            node_has_newline_cache: RefCell::new(vec![None; tree.next_id() as usize]),
            call_argument_expansion_profiles_cache: RefCell::new(vec![
                None;
                tree.next_id() as usize
            ]),
            argument_annotation_profile_cache: RefCell::new(vec![None; tree.next_id() as usize]),
            call_argument_facts_cache: RefCell::new(vec![None; tree.next_id() as usize]),
            call_argument_chain_force_expand_cache: RefCell::new(vec![
                None;
                tree.next_id() as usize
            ]),
            transparent_inner_expression_cache: RefCell::new(vec![None; tree.next_id() as usize]),
            expression_type_context_cache: vec![
                Cell::new(TYPE_CONTEXT_STATE_UNKNOWN);
                tree.next_id() as usize
            ],
            expression_template_interpolation_cache: vec![
                Cell::new(TYPE_CONTEXT_STATE_UNKNOWN);
                tree.next_id() as usize
            ],
            expression_type_conditional_ancestor_cache: vec![
                Cell::new(TYPE_CONTEXT_STATE_UNKNOWN);
                tree.next_id() as usize
            ],
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

        let span_str = self.get_span_str(span);
        let len = if span_str.is_ascii() {
            span_str.len()
        } else {
            span_str.chars().count()
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

        {
            let cache = self.node_span_char_len_cache.borrow();
            if let Some(len) = cache[node_index] {
                return len;
            }
        }

        let len = self.span_char_len(self.get_span(node_id));
        self.node_span_char_len_cache.borrow_mut()[node_index] = Some(len);

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

        {
            let cache = self.node_has_newline_cache.borrow();
            if let Some(has_newline) = cache[node_index] {
                return has_newline;
            }
        }

        let has_newline = self.has_newline(self.get_span(node_id));
        self.node_has_newline_cache.borrow_mut()[node_index] = Some(has_newline);

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
            let cache = self.transparent_inner_expression_cache.borrow();
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

        let mut cache = self.transparent_inner_expression_cache.borrow_mut();
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
            .expression_type_context_cache
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
        if let Some(cache_state) = self.expression_type_context_cache.get(node_index) {
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
                .expression_template_interpolation_cache
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
                .expression_template_interpolation_cache
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
                .expression_type_conditional_ancestor_cache
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
                .expression_type_conditional_ancestor_cache
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
        let node_index = node_id.id as usize;
        let state = self
            .annotation_presence_cache
            .get(node_index)
            .map(Cell::get)
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

        // lookup annotations once and cache both presence and metadata
        let node_has_annotations =
            state == ANNOTATION_STATE_PRESENT || self.tree.has_annotations(node_id.id);
        if !node_has_annotations {
            if let Some(presence_state) = self.annotation_presence_cache.get(node_index) {
                presence_state.set(ANNOTATION_STATE_NONE);
            }
            return None;
        }

        let annotation_data =
            CachedAnnotationData::from_ids(self.tree, self.tree.get_annotations(node_id.id));
        {
            let mut cache = self.annotation_ids_cache.borrow_mut();
            if node_index >= cache.len() {
                return None;
            }
            cache[node_index] = Some(annotation_data);
        }
        if let Some(presence_state) = self.annotation_presence_cache.get(node_index) {
            presence_state.set(ANNOTATION_STATE_CACHED);
        }

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
        let state = self
            .annotation_presence_cache
            .get(node_index)
            .map(Cell::get)
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
        if let Some(presence_state) = self.annotation_presence_cache.get(node_index) {
            presence_state.set(if has_annotation {
                ANNOTATION_STATE_PRESENT
            } else {
                ANNOTATION_STATE_NONE
            });
        }
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
        if let Some(profile) =
            self.cache_get_copy_entry(&self.argument_annotation_profile_cache, argument_id.id)
        {
            self.increment_counter("cache.argument_annotation_profile.hits", 1);
            return profile;
        }

        self.increment_counter("cache.argument_annotation_profile.misses", 1);
        let profile = self.compute_argument_annotation_profile(argument_id);
        self.cache_set_copy_entry(
            &self.argument_annotation_profile_cache,
            argument_id.id,
            profile,
        );

        profile
    }

    /// Return cached call argument expansion facts for one call expression node.
    #[inline]
    pub fn cached_call_argument_facts(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<CachedCallArgumentFacts> {
        self.cache_get_copy_entry(&self.call_argument_facts_cache, call_node_id.id)
    }

    /// Cache call argument expansion facts for one call expression node.
    #[inline]
    pub fn cache_call_argument_facts(
        &self,
        call_node_id: LocalNodeId<Expression>,
        facts: CachedCallArgumentFacts,
    ) {
        self.cache_set_copy_entry(&self.call_argument_facts_cache, call_node_id.id, facts);
    }

    /// Return cached chain call force-expand decision for one call expression node.
    #[inline]
    pub fn cached_call_argument_chain_force_expand(
        &self,
        call_node_id: LocalNodeId<Expression>,
    ) -> Option<bool> {
        self.cache_get_copy_entry(
            &self.call_argument_chain_force_expand_cache,
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
            &self.call_argument_chain_force_expand_cache,
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
            &self.call_argument_expansion_profiles_cache,
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
            &self.call_argument_expansion_profiles_cache,
            call_node_id.id,
            profiles,
        );
    }

    /// Compute annotation facts for one argument node.
    fn compute_argument_annotation_profile(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> CachedArgumentAnnotationProfile {
        if !self.has_annotation(argument_id) {
            return CachedArgumentAnnotationProfile::default();
        }

        let argument_span = self.get_span(argument_id);
        let mut profile = CachedArgumentAnnotationProfile::default();

        self.with_annotations(argument_id, |annotations| {
            for annotation_id in annotations {
                let annotation = self.tree.get::<Annotation>(*annotation_id);

                match annotation {
                    Annotation::Blank { .. } => {}
                    Annotation::Doc { position, .. }
                    | Annotation::Decorator { position, .. }
                    | Annotation::Comment { position, .. } => {
                        let Annotation::Comment { node, .. } = annotation else {
                            continue;
                        };
                        profile.has_comment = true;

                        let comment = self.tree.get::<Comment>(*node);
                        if comment.style != destack_ast::CommentStyle::Slash {
                            continue;
                        }

                        let annotation_span = self.get_span::<Annotation>(*annotation_id);
                        if annotation_span.start >= argument_span.end {
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
