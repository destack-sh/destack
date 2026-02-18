use super::*;

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
    /// Cached char lengths for repeated span width checks.
    pub span_char_len_by_span: RefCell<FxHashMap<Span, usize>>,
    /// Whether the file text is fully ASCII.
    pub source_is_ascii: bool,
    /// Cached newline byte offsets in file text.
    pub newline_offsets: OnceCell<Vec<u32>>,
    /// Cached newline checks for repeated span newline predicates.
    pub span_has_newline_by_span: RefCell<FxHashMap<Span, bool>>,
    /// Cached comment checks for repeated span comment predicates.
    pub span_has_comment_by_span: RefCell<FxHashMap<Span, bool>>,
    /// Dense formatter caches keyed by node id.
    pub(super) node_caches: FormatterNodeCaches,
    /// Cached sorted comment tokens for ignore-range scans.
    pub comment_tokens_sorted: OnceCell<Vec<TokenSpan>>,
    /// Comment spans for this file, sorted by start position.
    pub comment_spans: Vec<Span>,
    /// Line-comment spans for this file, sorted by start position.
    pub line_comment_spans: Vec<Span>,
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
        let mut comment_spans = Vec::new();
        let mut line_comment_spans = Vec::new();
        for token in tokens.iter().chain(side_tokens.iter()) {
            match token.token.ty {
                TokenType::LineComment | TokenType::DocLineComment => {
                    comment_spans.push(token.span);
                    line_comment_spans.push(token.span);
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    comment_spans.push(token.span);
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
            span_char_len_by_span: RefCell::new(FxHashMap::default()),
            source_is_ascii,
            newline_offsets: OnceCell::new(),
            span_has_newline_by_span: RefCell::new(FxHashMap::default()),
            span_has_comment_by_span: RefCell::new(FxHashMap::default()),
            node_caches,
            comment_tokens_sorted: OnceCell::new(),
            comment_spans,
            line_comment_spans,
            timings: timings_enabled.then(|| Rc::new(FormatterTimings::default())),
            has_ignore_directive_markers,
            has_template_literal_markers,
            instrumentation_enabled: timings_enabled,
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
