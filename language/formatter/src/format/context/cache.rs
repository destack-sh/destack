use crate::format::context::{
    Annotation, AnnotationPosition, Cell, Comment, Expression, FxHashMap, LocalNodeId,
    NODE_BOOL_STATE_UNKNOWN, NODE_SPAN_CHAR_LEN_UNKNOWN, OnceCell, RefCell, SmallVec,
    TYPE_CONTEXT_STATE_UNKNOWN,
};

/// Snapshot of formatter cache behavior counters.
#[derive(Debug, Clone, Copy, Default)]
pub struct FormatterCacheStatsSnapshot {
    /// Number of `span_text_by_span` hits.
    pub span_text_hits: usize,
    /// Number of `span_text_by_span` misses.
    pub span_text_misses: usize,
    /// Number of `span_has_newline_by_span` hits.
    pub span_has_newline_hits: usize,
    /// Number of `span_has_newline_by_span` misses.
    pub span_has_newline_misses: usize,
    /// Number of `span_has_comment_by_span` hits.
    pub span_has_comment_hits: usize,
    /// Number of `span_has_comment_by_span` misses.
    pub span_has_comment_misses: usize,
    /// Number of annotation cache hits.
    pub annotation_cache_hits: usize,
    /// Number of annotation cache misses.
    pub annotation_cache_misses: usize,
}

/// Shared formatter cache instrumentation counters.
#[derive(Debug, Default)]
pub struct FormatterCacheStatsCollector {
    /// Number of `span_text_by_span` hits.
    pub span_text_hits: Cell<usize>,
    /// Number of `span_text_by_span` misses.
    pub span_text_misses: Cell<usize>,
    /// Number of `span_has_newline_by_span` hits.
    pub span_has_newline_hits: Cell<usize>,
    /// Number of `span_has_newline_by_span` misses.
    pub span_has_newline_misses: Cell<usize>,
    /// Number of `span_has_comment_by_span` hits.
    pub span_has_comment_hits: Cell<usize>,
    /// Number of `span_has_comment_by_span` misses.
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
pub struct AnnotationData {
    /// Annotation ids attached to the node.
    pub ids: Vec<LocalNodeId<Annotation>>,
    /// Whether any annotation is non-blank.
    pub has_non_blank: bool,
    /// Whether any annotation is non-blank and not a boundary postfix comment.
    pub has_non_blank_non_boundary: bool,
    /// Whether any annotation is a prefix annotation.
    pub has_prefix: bool,
    /// Whether any annotation is a block prefix annotation.
    pub has_block_prefix: bool,
    /// Whether any annotation is a line prefix annotation.
    pub has_line_prefix: bool,
    /// Whether any annotation is an infix annotation.
    pub has_infix: bool,
    /// Whether any annotation is a non-blank infix annotation.
    pub has_non_blank_infix: bool,
    /// Whether any annotation is a postfix annotation.
    pub has_postfix: bool,
    /// Whether any annotation is a non-blank postfix annotation.
    pub has_non_blank_postfix: bool,
    /// Whether any annotation is a blank postfix annotation.
    pub has_blank_postfix: bool,
    /// Whether any annotation is a blank prefix annotation.
    pub has_blank_prefix: bool,
    /// Whether the first annotation is a blank prefix annotation.
    pub has_blank_prefix_first: bool,
    /// Whether any annotation is a boundary postfix comment.
    pub has_boundary_comment: bool,
}

impl AnnotationData {
    /// Build cached annotation metadata from annotation ids.
    pub fn from_ids(
        ids: Vec<LocalNodeId<Annotation>>,
        mut annotation_for: impl FnMut(LocalNodeId<Annotation>) -> Annotation,
    ) -> Self {
        let mut has_non_blank = false;
        let mut has_non_blank_non_boundary = false;
        let mut has_prefix = false;
        let mut has_block_prefix = false;
        let mut has_line_prefix = false;
        let mut has_infix = false;
        let mut has_non_blank_infix = false;
        let mut has_postfix = false;
        let mut has_non_blank_postfix = false;
        let mut has_blank_postfix = false;
        let mut has_blank_prefix = false;
        let mut has_blank_prefix_first = false;
        let mut has_boundary_comment = false;

        for (index, annotation_id) in ids.iter().enumerate() {
            let annotation = annotation_for(*annotation_id);
            let position = annotation.position();
            let is_non_blank = !matches!(annotation, Annotation::Blank { .. });

            if is_non_blank {
                has_non_blank = true;
            }

            if is_non_blank
                && !matches!(
                    annotation,
                    Annotation::Comment {
                        position: AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            {
                has_non_blank_non_boundary = true;
            }

            // general position flags
            if position == AnnotationPosition::BlockPrefix
                || position == AnnotationPosition::LinePrefix
            {
                has_prefix = true;
                if position == AnnotationPosition::BlockPrefix {
                    has_block_prefix = true;
                } else {
                    has_line_prefix = true;
                }
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
                if is_non_blank {
                    has_non_blank_postfix = true;
                } else {
                    has_blank_postfix = true;
                }
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

            if matches!(
                annotation,
                Annotation::Comment {
                    position: AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            ) {
                has_boundary_comment = true;
            }
        }

        Self {
            ids,
            has_non_blank,
            has_non_blank_non_boundary,
            has_prefix,
            has_block_prefix,
            has_line_prefix,
            has_infix,
            has_non_blank_infix,
            has_postfix,
            has_non_blank_postfix,
            has_blank_postfix,
            has_blank_prefix,
            has_blank_prefix_first,
            has_boundary_comment,
        }
    }
}

/// Cached argument annotation data used by hot call formatting paths.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArgumentAnnotationCache {
    /// Whether the argument has any comment annotation.
    pub has_comment: bool,
    /// Whether the argument has a trailing slash style comment annotation.
    pub has_line_comment: bool,
    /// Whether the argument has a slash style prefix comment annotation.
    pub has_prefix_line_comment: bool,
    /// Whether the argument has any non-blank prefix annotation.
    pub has_prefix_annotation: bool,
}

/// Cached separator-comment source attached to one argument.
#[derive(Debug, Clone, Default)]
pub struct SeparatorLineCommentSourceCache {
    /// The separator comment node ids in source order.
    pub comment_ids: SmallVec<[LocalNodeId<Comment>; 2]>,
    /// Whether the comment starts on its own line after the separator comma.
    pub is_own_line: bool,
    /// Whether source had a blank line between separator and first comment.
    pub has_blank_line_before_first_comment: bool,
    /// Whether comments were detached from the following argument prefix.
    pub detached_from_following_prefix: bool,
}

/// Cached call argument layout data keyed by call expression node id.
#[derive(Debug, Clone, Copy, Default)]
pub struct CallArgumentLayoutCache {
    /// Whether the call has a non-blank infix annotation.
    pub has_call_infix_annotations: bool,
    /// Whether source text around call argument boundaries contains line comments.
    pub has_boundary_comments: bool,
    /// Whether any dynamic argument has annotations.
    pub has_any_argument_annotation: bool,
    /// Whether all dynamic arguments are single-line and unannotated.
    pub all_single_line_and_unannotated: bool,
    /// Whether all dynamic arguments are compact, simple, and unannotated.
    pub all_compact_simple_unannotated: bool,
    /// Whether all leading arguments before the last are compact, simple, and unannotated.
    pub leading_arguments_are_compact_simple_unannotated: bool,
    /// Whether all leading arguments before the last are compact callback-tail candidates.
    pub leading_arguments_are_compact_callback_tail_candidates: bool,
    /// Whether all arguments can use the plain argument writer.
    pub all_plain_call_arguments: bool,
    /// Whether any argument has a line comment annotation.
    pub has_line_comment_annotations: bool,
    /// Whether any argument has a prefix line comment annotation.
    pub has_prefix_line_comment_annotations: bool,
    /// Whether any argument is a block callback.
    pub has_block_callback_argument: bool,
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
pub(crate) struct FormatterNodeCaches {
    /// Cached span char lengths for node ids.
    pub(crate) node_span_char_len: Vec<Cell<u32>>,
    /// Cached node span newline predicates keyed by node id.
    pub(crate) node_has_newline: Vec<Cell<u8>>,
    /// Cached call argument annotation data keyed by argument node id.
    pub(crate) argument_annotation_cache: Vec<Cell<Option<ArgumentAnnotationCache>>>,
    /// Cached call argument layout data keyed by call expression node id.
    pub(crate) call_argument_layout_cache: Vec<Cell<Option<CallArgumentLayoutCache>>>,
    /// Cached separator-comment sources keyed by argument node id.
    pub(crate) separator_line_comment_source:
        Vec<OnceCell<Option<SeparatorLineCommentSourceCache>>>,
    /// Cached chain call force-expand states keyed by call expression node id.
    pub(crate) call_argument_chain_force_expand: Vec<Cell<Option<bool>>>,
    /// Cached transparent inner expression ids keyed by expression node id.
    pub(crate) transparent_inner_expression: Vec<Cell<Option<LocalNodeId<Expression>>>>,
    /// Cached type-context states keyed by expression node id.
    pub(crate) expression_type_context: Vec<Cell<u8>>,
    /// Cached template interpolation ancestry states keyed by expression node id.
    pub(crate) expression_template_interpolation: Vec<Cell<u8>>,
    /// Cached type-conditional ancestry states keyed by expression node id.
    pub(crate) expression_type_conditional_ancestor: Vec<Cell<u8>>,
}

impl FormatterNodeCaches {
    /// Build all dense formatter node caches.
    pub(crate) fn new(node_count: usize) -> Self {
        Self {
            node_span_char_len: vec![Cell::new(NODE_SPAN_CHAR_LEN_UNKNOWN); node_count],
            node_has_newline: vec![Cell::new(NODE_BOOL_STATE_UNKNOWN); node_count],
            argument_annotation_cache: vec![Cell::new(None); node_count],
            call_argument_layout_cache: vec![Cell::new(None); node_count],
            separator_line_comment_source: std::iter::repeat_with(OnceCell::new)
                .take(node_count)
                .collect(),
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
