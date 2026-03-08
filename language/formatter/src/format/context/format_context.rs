use crate::format::context::source::token_keyword_map;
use crate::format::context::{
    ANNOTATION_STATE_NONE, ANNOTATION_STATE_PRESENT, AnnotationData, AnnotationPosition, Argument,
    Blank, Block, Cell, Comment, Declaration, Declarator, Decorator, DependencyItem, Doc,
    EnumField, Expression, File, Format, FormatContext, FormatResult, Formatter,
    FormatterCacheStatsCollector, FormatterCacheStatsSnapshot, FormatterCounterEntry,
    FormatterCountersCollector, FormatterNodeCaches, FormatterTimingEntry, FormatterTimingScope,
    FormatterTimingTag, FormatterTimings, FxHashMap, GroupId, ImmutableStringPool, Keyword,
    LocalNodeId, LocalNodeIdAny, MatchCase, Member, MultiSpan, Node, NodeParentIndex,
    NodeSourceMap, NodeTree, NodeTreeImpl, NodeType, OnceCell, Parameter, Pattern, PatternField,
    Property, Rc, RefCell, SeparatorLineCommentSourceCache, SmallVec, Span, TokenSpan, TokenType,
    WhereClause, formatter_annotation_projection, tag_for_node_type,
};
use destack_fir::format::FormatOptions;
use destack_fir::print::PrintOptions;
use destack_source::{IndentStyle, LanguageType, LineEnding};
use destack_workspace::{
    ArrowParentheses, FormatterOptions, ImportSortOrder, OrganizeImports, QuoteProperty,
    QuoteStyle, TrailingComma,
};

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
    /// Trailing comma rules for multi-line constructs.
    pub trailing_comma: TrailingComma = TrailingComma::All,
    /// Spaces inside object braces: `{ foo }` (true) vs `{foo}` (false).
    pub bracket_spacing: bool = true,
    /// Arrow function parentheses rules.
    pub arrow_parentheses: ArrowParentheses = ArrowParentheses::Always,
    /// Object property quoting rules.
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
            trim_trailing_whitespace: true,
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

/// The formatter implementation specialized for the Destack context.
pub type DestackFormatter<'ast, 'buf> = Formatter<'buf, DestackFormatContext<'ast>>;

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
    pub annotation_data_by_node_id: RefCell<Vec<Option<AnnotationData>>>,
    /// Cached annotation presence for node ids.
    pub annotation_state_by_node_id: Vec<Cell<u8>>,
    /// Cached source slices for repeated span lookups.
    pub span_text_by_span: RefCell<FxHashMap<Span, &'a str>>,
    /// Cached parsed identifier keywords by token span.
    pub token_keyword_by_span: RefCell<FxHashMap<Span, Option<Keyword>>>,
    /// Whether the file text is fully ASCII.
    pub source_is_ascii: bool,
    /// Cached newline byte offsets in file text.
    pub newline_offsets: OnceCell<Vec<u32>>,
    /// Cached newline checks for repeated span newline predicates.
    pub span_has_newline_by_span: RefCell<FxHashMap<Span, bool>>,
    /// Cached comment checks for repeated span comment predicates.
    pub span_has_comment_by_span: RefCell<FxHashMap<Span, bool>>,
    /// Dense formatter caches keyed by node id.
    pub(crate) node_caches: FormatterNodeCaches,
    /// Cached sorted comment tokens for ignore-range scans.
    pub comment_tokens_sorted: OnceCell<Vec<TokenSpan>>,
    /// Comment spans for this file, sorted by start position.
    pub comment_spans: Vec<Span>,
    /// Line-comment spans for this file, sorted by start position.
    pub line_comment_spans: Vec<Span>,
    /// Optional formatter timing collector.
    pub timings: Option<FormatterTimings>,
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
        Self::new_with_instrumentation(options, artifacts, false, false)
    }

    /// Construct a formatting context from parse artifacts with optional timing collection.
    pub fn new_with_timings(
        options: DestackFormatOptions,
        artifacts: DestackFormatArtifacts<'a>,
        timings_enabled: bool,
    ) -> Self {
        Self::new_with_instrumentation(options, artifacts, timings_enabled, timings_enabled)
    }

    /// Construct a formatting context from parse artifacts with optional timing and counter collection.
    pub fn new_with_instrumentation(
        options: DestackFormatOptions,
        artifacts: DestackFormatArtifacts<'a>,
        timings_enabled: bool,
        instrumentation_enabled: bool,
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
        let token_keyword_by_span = token_keyword_map(file, tokens, side_tokens);
        let (formatter_annotation_entries, formatter_annotation_ids_by_node_id) =
            formatter_annotation_projection(file, tree, tokens, &parents, &token_keyword_by_span);
        let node_count = tree.next_id() as usize;
        let node_caches = FormatterNodeCaches::new(node_count);
        let source_is_ascii = file.text().is_ascii();
        let mut has_ignore_directive_markers = false;
        let mut has_template_literal_markers = false;
        let mut comment_spans = Vec::new();
        let mut line_comment_spans = Vec::new();
        for token in tokens.iter().chain(side_tokens.iter()) {
            if matches!(
                token.token.ty,
                TokenType::TemplateStringStart
                    | TokenType::TemplateStringMiddle
                    | TokenType::TemplateStringEnd
                    | TokenType::TemplateString
            ) {
                has_template_literal_markers = true;
            }

            match token.token.ty {
                TokenType::LineComment | TokenType::DocLineComment => {
                    comment_spans.push(token.span);
                    line_comment_spans.push(token.span);

                    if !has_ignore_directive_markers {
                        let raw = file.span_str(token.span);
                        has_ignore_directive_markers =
                            crate::format::directive::is_any_ignore_directive_comment(raw);
                    }
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    comment_spans.push(token.span);

                    if !has_ignore_directive_markers {
                        let raw = file.span_str(token.span);
                        has_ignore_directive_markers =
                            crate::format::directive::is_any_ignore_directive_comment(raw);
                    }
                }
                _ => {}
            }
        }
        comment_spans.sort_by_key(|span| span.start);
        line_comment_spans.sort_by_key(|span| span.start);

        let annotation_state_by_node_id = vec![Cell::new(ANNOTATION_STATE_NONE); node_count];
        for (node_id, annotation_ids) in formatter_annotation_ids_by_node_id.iter().enumerate() {
            if annotation_ids.is_empty() {
                continue;
            }

            if let Some(annotation_state) = annotation_state_by_node_id.get(node_id) {
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
            annotation_data_by_node_id: RefCell::new(vec![None; node_count]),
            annotation_state_by_node_id,
            span_text_by_span: RefCell::new(FxHashMap::default()),
            token_keyword_by_span: RefCell::new(token_keyword_by_span),
            source_is_ascii,
            newline_offsets: OnceCell::new(),
            span_has_newline_by_span: RefCell::new(FxHashMap::default()),
            span_has_comment_by_span: RefCell::new(FxHashMap::default()),
            node_caches,
            comment_tokens_sorted: OnceCell::new(),
            comment_spans,
            line_comment_spans,
            timings: timings_enabled.then(FormatterTimings::default),
            has_ignore_directive_markers,
            has_template_literal_markers,
            instrumentation_enabled,
            file_ignore_applied: Rc::new(Cell::new(false)),
            cache_stats: Rc::new(FormatterCacheStatsCollector::default()),
            counters: Rc::new(FormatterCountersCollector::default()),
        }
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

impl<'a> DestackFormatContext<'a> {
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
    #[cfg(feature = "timings")]
    pub fn increment_counter(&self, name: &'static str, delta: usize) {
        if !self.instrumentation_enabled {
            return;
        }
        self.counters.increment(name, delta);
    }

    /// Increment a formatter instrumentation counter.
    #[inline]
    #[cfg(not(feature = "timings"))]
    pub fn increment_counter(&self, _name: &'static str, _delta: usize) {}

    /// Record one best fitting evaluation for a logical formatter region.
    #[inline]
    #[cfg(feature = "timings")]
    pub fn record_best_fitting(&self, label: &'static str, variants: usize) {
        self.increment_counter("best_fitting.calls.total", 1);
        self.increment_counter("best_fitting.variants.total", variants);
        self.increment_counter(label, 1);
    }

    /// Record one best fitting evaluation for a logical formatter region.
    #[inline]
    #[cfg(not(feature = "timings"))]
    pub fn record_best_fitting(&self, _label: &'static str, _variants: usize) {}

    /// Snapshot formatter instrumentation counters.
    #[inline]
    #[cfg(feature = "timings")]
    pub fn counter_snapshot(&self) -> Vec<FormatterCounterEntry> {
        self.counters.snapshot()
    }

    /// Snapshot formatter instrumentation counters.
    #[inline]
    #[cfg(not(feature = "timings"))]
    pub fn counter_snapshot(&self) -> Vec<FormatterCounterEntry> {
        Vec::new()
    }

    /// Resolve one cached separator-comment source for one argument.
    #[inline]
    pub fn separator_line_comment_source_cache(
        &self,
        argument_id: LocalNodeId<Argument>,
        compute: impl FnOnce() -> Option<SeparatorLineCommentSourceCache>,
    ) -> Option<SeparatorLineCommentSourceCache> {
        let Some(cache_cell) = self
            .node_caches
            .separator_line_comment_source
            .get(argument_id.id as usize)
        else {
            return compute();
        };

        if let Some(cached) = cache_cell.get() {
            self.increment_counter("call.arguments.separator_source.cache.hits", 1);
            return cached.clone();
        }

        self.increment_counter("call.arguments.separator_source.cache.misses", 1);
        let cached = compute();
        let _ = cache_cell.set(cached.clone());
        cached
    }

    /// Return whether one cached separator-comment source exists for one argument.
    #[inline]
    pub fn has_separator_line_comment_source(
        &self,
        argument_id: LocalNodeId<Argument>,
        compute: impl FnOnce() -> Option<SeparatorLineCommentSourceCache>,
    ) -> bool {
        let Some(cache_cell) = self
            .node_caches
            .separator_line_comment_source
            .get(argument_id.id as usize)
        else {
            return compute().is_some();
        };

        if let Some(cached) = cache_cell.get() {
            self.increment_counter("call.arguments.separator_source.cache.hits", 1);
            return cached.is_some();
        }

        self.increment_counter("call.arguments.separator_source.cache.misses", 1);
        let cached = compute();
        let exists = cached.is_some();
        let _ = cache_cell.set(cached);
        exists
    }
}

/// Format one typed AST node with full context.
pub(crate) trait FormatNode<'a, T: Node>
where
    DestackFormatContext<'a>: FormatContext,
{
    /// Format one AST node id.
    fn format_node(
        &self,
        node_id: LocalNodeId<T>,
        f: &mut DestackFormatter<'a, '_>,
    ) -> FormatResult<()>;
}

/// Implement formatter dispatch for typed local node ids.
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

/// Implement formatter dispatch for dynamically typed local node ids.
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
                let node = context.annotation(node_id);
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
