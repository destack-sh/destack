use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use destack_serde::Reflect;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::interval::IntervalTree;
use crate::{ByteRange, FileId, Span};

/// The type of node search to perform.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum NodeSearchMode {
    /// Search for the biggest outermost node that matches.
    BiggestOutermost,
    /// Search for the smallest outermost node that matches.
    SmallestOutermost,
    /// Search for the smallest innermost node that matches.
    SmallestInnermost,
}

/// The type of span for a node.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum NodeSpanType {
    /// The enclosing span of a node.
    Enclosing,
    /// The main span of a node (usually its identifier).
    Main,
    /// The head span of a node.
    Head,
    /// One boundary span owned by a node.
    Boundary(NodeSpanBoundary),
    /// One named region span within a node.
    Region(NodeSpanRegion),
    /// One indexed item span within a node.
    ListItem(NodeSpanList, u16),
}

/// A source boundary owned by one node.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum NodeSpanBoundary {
    /// The leading owned prefix span of a node.
    Leading,
    /// The leading operator span of a node.
    LeadingOperator,
    /// The trailing owned suffix span of a node.
    Trailing,
}

/// A named source region within one node.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum NodeSpanRegion {
    /// The opening element span of a compound node.
    Opening,
    /// The generic parameter container span of a function-like or declaration node.
    GenericParameters,
    /// The parameter container span of a function-like node.
    Parameters,
    /// The body container span of a function-like node.
    Body,
    /// The statement source span of a statement-position expression.
    Statement,
    /// A clause span within a node.
    Clause,
    /// A prelude span within a node.
    Prelude,
    /// A name span within a node.
    Name,
    /// A value span within a node.
    Value,
    /// The type declaration span of a node.
    Type,
    /// Parentheses around a value or type expression.
    Parentheses,
    /// A tree expression container around a value expression.
    TreeContainer,
}

/// An indexed source list within one node.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum NodeSpanList {
    /// One generic ordered source segment.
    Segment,
    /// One source entry.
    Entry,
}

/// One typed node span keyed by source node id and span kind.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct NodeSpanKey {
    /// The source node id that owns this node span.
    pub source_id: u32,
    /// The span kind within that source node.
    pub span_type: NodeSpanType,
}

impl NodeSpanKey {
    /// Build one typed node span key.
    pub fn new(source_id: u32, span_type: NodeSpanType) -> Self {
        Self {
            source_id,
            span_type,
        }
    }
}

/// A contiguous node id run that shares one source file.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct SourceFileRun {
    /// The first node id covered by the run.
    first_node_id: u32,
    /// The file shared by the run.
    file: FileId,
}

/// One source range keyed by source node id.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct NodeRange {
    /// The source node id.
    node_id: u32,
    /// The file-local source range.
    range: ByteRange,
}

/// One sparse side range.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct NodeRangeEntry {
    /// The node and span kind.
    key: NodeSpanKey,
    /// The file-local source range.
    range: ByteRange,
}

/// Position lookup index for all source files in one tree.
#[derive(Debug, Clone)]
struct PositionIndex {
    /// The interval trees by source file.
    file_trees: Vec<FilePositionIndex>,
}

impl PositionIndex {
    /// Build one position index from a source index.
    fn build(source_index: &SourceIndex) -> Self {
        let mut file_intervals: Vec<(FileId, Vec<(u32, u32, u32)>)> = Vec::new();
        let mut file_index_by_id = FxHashMap::default();

        for (index, range) in source_index.enclosing_ranges.iter().copied().enumerate() {
            let node_id = index as u32;
            let file = source_index.file_for_node(node_id);
            let interval = (range.start, range.end, node_id);

            let bucket = match file_index_by_id.get(&file).copied() {
                Some(bucket) => bucket,
                None => {
                    let bucket = file_intervals.len();
                    file_intervals.push((file, Vec::new()));
                    file_index_by_id.insert(file, bucket);
                    bucket
                }
            };
            file_intervals[bucket].1.push(interval);
        }

        let mut file_trees = file_intervals
            .into_iter()
            .map(|(file, intervals)| FilePositionIndex {
                file,
                tree: IntervalTree::build(intervals),
            })
            .collect::<Vec<_>>();
        file_trees.sort_unstable_by_key(|file_tree| file_tree.file);

        Self { file_trees }
    }

    /// Return the interval tree for one source file.
    fn tree_for_file(&self, file: FileId) -> Option<&IntervalTree> {
        self.file_trees
            .binary_search_by_key(&file, |file_tree| file_tree.file)
            .ok()
            .map(|index| &self.file_trees[index].tree)
    }
}

/// Position lookup index for one source file.
#[derive(Debug, Clone)]
struct FilePositionIndex {
    /// The indexed source file.
    file: FileId,
    /// The nested interval tree for this file.
    tree: IntervalTree,
}

/// Source range and position index for tree nodes.
#[derive(Debug)]
pub struct SourceIndex {
    /// Source file runs for node ranges.
    file_runs: Vec<SourceFileRun>,
    /// Enclosing ranges of all nodes, indexed by global node id.
    enclosing_ranges: Vec<ByteRange>,
    /// Main ranges keyed by global node id.
    main_ranges: Vec<NodeRange>,
    /// Type ranges keyed by global node id.
    type_ranges: Vec<NodeRange>,
    /// Extra side ranges for non-main/type ranges.
    side_ranges: Vec<NodeRangeEntry>,
    /// Position index for O(log n + k) enclosing span queries.
    /// Built lazily on first lookup and invalidated on enclosing span mutations.
    position_index: RwLock<Option<PositionIndex>>,
    /// Whether the position index cache is currently built.
    position_index_ready: AtomicBool,
}

impl Clone for SourceIndex {
    fn clone(&self) -> Self {
        Self {
            file_runs: self.file_runs.clone(),
            enclosing_ranges: self.enclosing_ranges.clone(),
            main_ranges: self.main_ranges.clone(),
            type_ranges: self.type_ranges.clone(),
            side_ranges: self.side_ranges.clone(),
            position_index: RwLock::new(None),
            position_index_ready: AtomicBool::new(false),
        }
    }
}

/// Serialized source index shape.
#[derive(Serialize, Deserialize, Reflect)]
struct SourceIndexData {
    enclosing_spans: Vec<Span>,
    #[serde(default)]
    main_spans: Vec<Option<Span>>,
    #[serde(default)]
    type_spans: Vec<Option<Span>>,
    #[serde(default)]
    side_spans: FxHashMap<NodeSpanKey, Span>,
}

impl Reflect for SourceIndex {
    fn reflect(registry: &mut destack_serde::SchemaRegistry) -> destack_serde::SchemaRef {
        SourceIndexData::reflect(registry)
    }
}

/// Borrowed source index serialization view.
#[derive(Serialize)]
struct SourceIndexRef<'a> {
    enclosing_spans: &'a [Span],
    main_spans: Vec<Option<Span>>,
    type_spans: Vec<Option<Span>>,
    side_spans: FxHashMap<NodeSpanKey, Span>,
}

impl Serialize for SourceIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let enclosing_spans = self.collect_enclosing_spans();
        let mut main_spans = Vec::with_capacity(self.enclosing_ranges.len());
        let mut type_spans = Vec::with_capacity(self.enclosing_ranges.len());

        // expand sparse ranges under each owning file
        for index in 0..self.enclosing_ranges.len() {
            main_spans.push(
                self.get_main_range(index as u32)
                    .map(|range| self.expand_range(index as u32, range)),
            );
            type_spans.push(
                self.get_type_range(index as u32)
                    .map(|range| self.expand_range(index as u32, range)),
            );
        }
        let side_spans = self.collect_side_spans();

        let data = SourceIndexRef {
            enclosing_spans: &enclosing_spans,
            main_spans,
            type_spans,
            side_spans,
        };
        data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SourceIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = SourceIndexData::deserialize(deserializer)?;
        let enclosing_len = data.enclosing_spans.len();
        let (file_runs, enclosing_ranges) = Self::split_spans(data.enclosing_spans);
        // restore main ranges
        let mut main_ranges = Vec::new();
        for index in 0..enclosing_len {
            if let Some(span) = data.main_spans.get(index).copied().flatten() {
                main_ranges.push(NodeRange {
                    node_id: index as u32,
                    range: span.range(),
                });
            }
        }

        // restore type ranges
        let mut type_ranges = Vec::new();
        for index in 0..enclosing_len {
            if let Some(span) = data.type_spans.get(index).copied().flatten() {
                type_ranges.push(NodeRange {
                    node_id: index as u32,
                    range: span.range(),
                });
            }
        }

        // restore arbitrary side ranges in key order
        let mut side_ranges: Vec<NodeRangeEntry> = data
            .side_spans
            .into_iter()
            .map(|(key, span)| NodeRangeEntry {
                key,
                range: span.range(),
            })
            .collect();
        side_ranges.sort_unstable_by_key(|entry| entry.key);

        Ok(Self {
            file_runs,
            enclosing_ranges,
            main_ranges,
            type_ranges,
            side_ranges,
            position_index: RwLock::new(None),
            position_index_ready: AtomicBool::new(false),
        })
    }
}

impl Default for SourceIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of finding enclosing spans at a position.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct EnclosingSpan {
    /// The source node id for this enclosing span.
    pub source_id: u32,
    /// The distance to the target span.
    pub distance: u32,
    /// The length of the enclosing span.
    pub length: u32,
    /// The enclosing span.
    pub span: Span,
}

impl SourceIndex {
    /// Create an empty source index.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create an empty source index with room for node spans.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            file_runs: Vec::new(),
            enclosing_ranges: Vec::with_capacity(capacity),
            main_ranges: Vec::new(),
            type_ranges: Vec::new(),
            side_ranges: Vec::new(),
            position_index: RwLock::new(None),
            position_index_ready: AtomicBool::new(false),
        }
    }

    /// Return whether this index has an enclosing span for a node id.
    #[inline]
    pub fn contains_node(&self, node_id: u32) -> bool {
        (node_id as usize) < self.enclosing_ranges.len()
    }

    /// Invalidate cached position index after enclosing span mutations.
    #[inline]
    fn invalidate_position_index(&mut self) {
        if !self.position_index_ready.load(Ordering::Relaxed) {
            return;
        }

        let position_index = match self.position_index.get_mut() {
            Ok(position_index) => position_index,
            Err(error) => error.into_inner(),
        };
        *position_index = None;
        self.position_index_ready.store(false, Ordering::Relaxed);
    }

    /// Ensure the position index exists for enclosing span lookups.
    #[inline]
    fn ensure_position_index(&self) {
        if self.position_index_ready.load(Ordering::Relaxed) {
            return;
        }

        {
            let position_index = match self.position_index.read() {
                Ok(position_index) => position_index,
                Err(error) => error.into_inner(),
            };
            if position_index.is_some() {
                self.position_index_ready.store(true, Ordering::Relaxed);
                return;
            }
        }

        let index = PositionIndex::build(self);
        let mut position_index = match self.position_index.write() {
            Ok(position_index) => position_index,
            Err(error) => error.into_inner(),
        };
        if position_index.is_none() {
            *position_index = Some(index);
        }

        self.position_index_ready.store(true, Ordering::Relaxed);
    }

    /// Append a span to the index.
    #[inline]
    pub fn append(&mut self, span: Span) {
        self.begin_file_run(span.file);
        self.enclosing_ranges.push(span.range());
        self.invalidate_position_index();
    }

    /// Begin appending parsed nodes from one source file.
    #[inline]
    pub fn begin_source_file(&mut self, file: FileId) {
        self.begin_file_run(file);
        self.invalidate_position_index();
    }

    /// Append one parsed node range before source position lookups begin.
    #[inline]
    pub fn append_parsed(&mut self, range: ByteRange) {
        debug_assert!(!self.file_runs.is_empty());
        debug_assert!(!self.position_index_ready.load(Ordering::Relaxed));
        self.enclosing_ranges.push(range);
    }

    /// Set the span for a node.
    #[inline]
    pub fn set(&mut self, node_id: u32, span: Span) {
        let index = node_id as usize;
        let previous_range = self.enclosing_ranges[index];
        let previous_file = self.file_for_node(node_id);
        let file = span.file;
        let range = span.range();
        self.enclosing_ranges[index] = range;
        self.set_file_for_node(node_id, file);

        if previous_range != range || previous_file != file {
            self.invalidate_position_index();
        }
    }

    /// Set the file-local byte range for one node.
    #[inline]
    pub fn set_range(&mut self, node_id: u32, range: ByteRange) {
        let index = node_id as usize;
        let previous = self.enclosing_ranges[index];
        self.enclosing_ranges[index] = range;

        if previous != range {
            self.invalidate_position_index();
        }
    }

    /// Prune spans from the index after one mark.
    #[inline]
    pub fn prune_from(&mut self, retained_node_count: usize, first_pruned_node_id: u32) {
        self.enclosing_ranges.truncate(retained_node_count);

        // truncate each ordered sparse index at the first pruned node
        let file_run_count = self
            .file_runs
            .partition_point(|run| (run.first_node_id as usize) < retained_node_count);
        self.file_runs.truncate(file_run_count);

        let main_range_count = self
            .main_ranges
            .partition_point(|entry| (entry.node_id as usize) < retained_node_count);
        self.main_ranges.truncate(main_range_count);

        let type_range_count = self
            .type_ranges
            .partition_point(|entry| (entry.node_id as usize) < retained_node_count);
        self.type_ranges.truncate(type_range_count);

        let side_range_count = self
            .side_ranges
            .partition_point(|entry| entry.key.source_id < first_pruned_node_id);
        self.side_ranges.truncate(side_range_count);

        self.invalidate_position_index();
    }

    /// Build interval tree for fast enclosing span lookups.
    /// You do not need to call this explicitly: lookups build it lazily.
    pub fn build_position_index(&mut self) {
        self.ensure_position_index();
    }

    /// Set a side span for a node.
    #[inline]
    pub fn set_side(&mut self, node_id: u32, span_type: NodeSpanType, span: Span) {
        debug_assert!(self.contains_node(node_id));
        debug_assert_eq!(self.file_for_node(node_id), span.file);

        match span_type {
            NodeSpanType::Main => {
                self.set_main_range(node_id, span.range());
            }
            NodeSpanType::Region(NodeSpanRegion::Type) => {
                self.set_type_range(node_id, span.range());
            }
            _ => {
                let key = NodeSpanKey::new(node_id, span_type);
                self.set_sparse_side_range(key, span.range());
            }
        }
    }

    /// Set a file-local side range for one node.
    #[inline]
    pub fn set_side_range(&mut self, node_id: u32, span_type: NodeSpanType, range: ByteRange) {
        debug_assert!(self.contains_node(node_id));
        match span_type {
            NodeSpanType::Main => self.set_main_range(node_id, range),
            NodeSpanType::Region(NodeSpanRegion::Type) => self.set_type_range(node_id, range),
            _ => self.set_sparse_side_range(NodeSpanKey::new(node_id, span_type), range),
        }
    }

    /// Get a side span for a node, if it has one.
    #[inline]
    pub fn get_side(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        match span_type {
            NodeSpanType::Main => self
                .get_main_range(node_id)
                .map(|range| self.expand_range(node_id, range)),
            NodeSpanType::Region(NodeSpanRegion::Type) => self
                .get_type_range(node_id)
                .map(|range| self.expand_range(node_id, range)),
            _ => self
                .get_sparse_side_range(NodeSpanKey::new(node_id, span_type))
                .map(|range| self.expand_range(node_id, range)),
        }
    }

    /// Get a file-local side range for one node when present.
    #[inline]
    pub fn get_side_range(&self, node_id: u32, span_type: NodeSpanType) -> Option<ByteRange> {
        match span_type {
            NodeSpanType::Main => self.get_main_range(node_id),
            NodeSpanType::Region(NodeSpanRegion::Type) => self.get_type_range(node_id),
            _ => self.get_sparse_side_range(NodeSpanKey::new(node_id, span_type)),
        }
    }

    /// Find the innermost side span of one kind that fully contains a span.
    pub fn find_innermost_side_owner(
        &self,
        span: Span,
        span_type: NodeSpanType,
    ) -> Option<NodeSpanKey> {
        self.side_ranges
            .iter()
            .filter_map(|entry| {
                (entry.key.span_type == span_type
                    && self.file_for_node(entry.key.source_id) == span.file
                    && entry.range.start <= span.start
                    && entry.range.end >= span.end)
                    .then_some((entry.key, entry.range))
            })
            .min_by_key(|(_, node_span)| node_span.end - node_span.start)
            .map(|(key, _)| key)
    }

    /// Find the innermost non-enclosing node span that fully contains a span.
    pub fn find_innermost_node_span_owner(&self, span: Span) -> Option<NodeSpanKey> {
        let mut best_owner = None;
        let mut best_length = u32::MAX;

        // main spans
        for entry in &self.main_ranges {
            if self.file_for_node(entry.node_id) == span.file
                && entry.range.start <= span.start
                && entry.range.end >= span.end
            {
                let span_length = entry.range.end - entry.range.start;
                if span_length < best_length {
                    best_owner = Some(NodeSpanKey::new(entry.node_id, NodeSpanType::Main));
                    best_length = span_length;
                }
            }
        }

        // type spans
        for entry in &self.type_ranges {
            if self.file_for_node(entry.node_id) == span.file
                && entry.range.start <= span.start
                && entry.range.end >= span.end
            {
                let span_length = entry.range.end - entry.range.start;
                if span_length < best_length {
                    best_owner = Some(NodeSpanKey::new(
                        entry.node_id,
                        NodeSpanType::Region(NodeSpanRegion::Type),
                    ));
                    best_length = span_length;
                }
            }
        }

        // sparse side spans
        for entry in &self.side_ranges {
            if self.file_for_node(entry.key.source_id) != span.file
                || entry.range.start > span.start
                || entry.range.end < span.end
            {
                continue;
            }

            let span_length = entry.range.end - entry.range.start;
            if span_length < best_length {
                best_owner = Some(entry.key);
                best_length = span_length;
            }
        }

        best_owner
    }

    /// Find the innermost enclosing owner that fully contains a span.
    pub fn find_innermost_enclosing_owner(&self, span: Span) -> Option<NodeSpanKey> {
        let end_inclusive = span.end.saturating_sub(1);

        self.get_enclosing_spans(span.file, span.start, end_inclusive)
            .into_iter()
            .filter(|enclosing| {
                enclosing.span.file == span.file
                    && enclosing.span.start <= span.start
                    && enclosing.span.end >= span.end
            })
            .min_by_key(|enclosing| enclosing.length)
            .map(|enclosing| NodeSpanKey::new(enclosing.source_id, NodeSpanType::Enclosing))
    }

    /// Find the nearest enclosing owner that begins after a position.
    pub fn find_nearest_enclosing_owner_after(
        &self,
        file: FileId,
        position: u32,
    ) -> Option<NodeSpanKey> {
        self.enclosing_ranges
            .iter()
            .enumerate()
            .filter(|(index, span)| {
                self.file_for_node(*index as u32) == file && span.start >= position
            })
            .min_by_key(|(_, span)| (span.start - position, span.end - span.start))
            .map(|(index, _)| NodeSpanKey::new(index as u32, NodeSpanType::Enclosing))
    }

    /// Find the nearest enclosing owner that ends before a position.
    pub fn find_nearest_enclosing_owner_before(
        &self,
        file: FileId,
        position: u32,
    ) -> Option<NodeSpanKey> {
        self.enclosing_ranges
            .iter()
            .enumerate()
            .filter(|(index, span)| {
                self.file_for_node(*index as u32) == file && span.end <= position
            })
            .min_by_key(|(_, span)| (position - span.end, span.end - span.start))
            .map(|(index, _)| NodeSpanKey::new(index as u32, NodeSpanType::Enclosing))
    }

    /// Get a side span or the enclosing span if no side span is set.
    #[inline]
    pub fn get_side_or_enclosing(&self, node_id: u32, span_type: NodeSpanType) -> Span {
        self.get_side(node_id, span_type)
            .unwrap_or_else(|| self.get(node_id))
    }

    /// Get a side span or a main span or an enclosing span if no side or main span is set.
    #[inline]
    pub fn get_side_or_main_or_enclosing(&self, node_id: u32, span_type: NodeSpanType) -> Span {
        self.get_side(node_id, span_type)
            .unwrap_or_else(|| self.get_main(node_id).unwrap_or_else(|| self.get(node_id)))
    }

    /// Set the main span for a node.
    #[inline]
    pub fn set_main(&mut self, node_id: u32, span: Span) {
        self.set_side(node_id, NodeSpanType::Main, span);
    }

    /// Get the main span for a node, if it has one.
    #[inline]
    pub fn get_main(&self, node_id: u32) -> Option<Span> {
        self.get_side(node_id, NodeSpanType::Main)
    }

    /// Get the main span or the enclosing span if no main span is set.
    #[inline]
    pub fn get_main_or_enclosing(&self, node_id: u32) -> Span {
        self.get_main(node_id).unwrap_or_else(|| self.get(node_id))
    }

    /// Get the span for a node by its id.
    #[inline]
    pub fn get(&self, node_id: u32) -> Span {
        self.expand_range(node_id, self.enclosing_ranges[node_id as usize])
    }

    /// Get the file-local byte range for one node by its id.
    #[inline]
    pub fn get_range(&self, node_id: u32) -> ByteRange {
        self.enclosing_ranges[node_id as usize]
    }

    /// Get the span for a node by its id when it is present.
    #[inline]
    pub fn try_get(&self, node_id: u32) -> Option<Span> {
        self.enclosing_ranges
            .get(node_id as usize)
            .copied()
            .map(|range| self.expand_range(node_id, range))
    }

    /// Get all enclosing spans containing the given range.
    ///
    /// Uses interval tree for O(log n + k) lookup where k is the number of enclosing spans.
    pub fn get_enclosing_spans(
        &self,
        file: FileId,
        start: u32,
        end_inclusive: u32,
    ) -> Vec<EnclosingSpan> {
        self.ensure_position_index();

        if self.enclosing_ranges.is_empty() {
            return Vec::new();
        }

        let position_index = match self.position_index.read() {
            Ok(position_index) => position_index,
            Err(error) => error.into_inner(),
        };
        let position_index = position_index
            .as_ref()
            .expect("position index must exist after ensure_position_index");
        let Some(tree) = position_index.tree_for_file(file) else {
            return Vec::new();
        };

        tree.query_containing(start, end_inclusive)
            .into_iter()
            .map(|(range_start, range_end, node_id, length)| {
                let distance = start.abs_diff(range_start) + range_end.abs_diff(end_inclusive);
                EnclosingSpan {
                    source_id: node_id,
                    distance,
                    length,
                    span: Span {
                        file,
                        start: range_start,
                        end: range_end,
                    },
                }
            })
            .collect()
    }

    /// Visit all enclosing spans containing the given range.
    pub fn visit_enclosing_spans(
        &self,
        file: FileId,
        start: u32,
        end_inclusive: u32,
        mut visit: impl FnMut(EnclosingSpan),
    ) {
        self.ensure_position_index();

        if self.enclosing_ranges.is_empty() {
            return;
        }

        let position_index = match self.position_index.read() {
            Ok(position_index) => position_index,
            Err(error) => error.into_inner(),
        };
        let position_index = position_index
            .as_ref()
            .expect("position index must exist after ensure_position_index");
        let Some(tree) = position_index.tree_for_file(file) else {
            return;
        };

        tree.visit_containing(
            start,
            end_inclusive,
            |range_start, range_end, node_id, length| {
                let distance = start.abs_diff(range_start) + range_end.abs_diff(end_inclusive);
                visit(EnclosingSpan {
                    source_id: node_id,
                    distance,
                    length,
                    span: Span {
                        file,
                        start: range_start,
                        end: range_end,
                    },
                });
            },
        );
    }

    /// Rebind all spans in the index to one file id.
    pub fn rebind_file(&mut self, file: crate::FileId) {
        if self.enclosing_ranges.is_empty() {
            self.file_runs.clear();
        } else {
            self.file_runs.clear();
            self.file_runs.push(SourceFileRun {
                first_node_id: 0,
                file,
            });
        }

        self.invalidate_position_index();
    }

    /// Split full spans into source file runs and byte ranges.
    fn split_spans(spans: Vec<Span>) -> (Vec<SourceFileRun>, Vec<ByteRange>) {
        let mut file_runs = Vec::new();
        let mut ranges = Vec::with_capacity(spans.len());

        for span in spans {
            let node_id = ranges.len() as u32;
            if file_runs.last().map(|run: &SourceFileRun| run.file) != Some(span.file) {
                file_runs.push(SourceFileRun {
                    first_node_id: node_id,
                    file: span.file,
                });
            }

            ranges.push(span.range());
        }

        (file_runs, ranges)
    }

    /// Collect full enclosing spans for serialization.
    fn collect_enclosing_spans(&self) -> Vec<Span> {
        self.enclosing_ranges
            .iter()
            .copied()
            .enumerate()
            .map(|(index, range)| self.expand_range(index as u32, range))
            .collect()
    }

    /// Collect full sparse side spans for serialization.
    fn collect_side_spans(&self) -> FxHashMap<NodeSpanKey, Span> {
        self.side_ranges
            .iter()
            .map(|entry| {
                (
                    entry.key,
                    self.expand_range(entry.key.source_id, entry.range),
                )
            })
            .collect()
    }

    /// Return one type range by source node id.
    fn get_type_range(&self, node_id: u32) -> Option<ByteRange> {
        self.type_ranges
            .binary_search_by_key(&node_id, |entry| entry.node_id)
            .ok()
            .map(|index| self.type_ranges[index].range)
    }

    /// Return one main range by source node id.
    fn get_main_range(&self, node_id: u32) -> Option<ByteRange> {
        self.main_ranges
            .binary_search_by_key(&node_id, |entry| entry.node_id)
            .ok()
            .map(|index| self.main_ranges[index].range)
    }

    /// Set one main range by source node id.
    fn set_main_range(&mut self, node_id: u32, range: ByteRange) {
        if let Some(last) = self.main_ranges.last_mut() {
            if last.node_id == node_id {
                last.range = range;
                return;
            }
            if last.node_id < node_id {
                self.main_ranges.push(NodeRange { node_id, range });
                return;
            }
        }

        match self
            .main_ranges
            .binary_search_by_key(&node_id, |entry| entry.node_id)
        {
            Ok(index) => self.main_ranges[index].range = range,
            Err(index) => self.main_ranges.insert(index, NodeRange { node_id, range }),
        }
    }

    /// Set one type range by source node id.
    fn set_type_range(&mut self, node_id: u32, range: ByteRange) {
        if let Some(last) = self.type_ranges.last_mut() {
            if last.node_id == node_id {
                last.range = range;
                return;
            }
            if last.node_id < node_id {
                self.type_ranges.push(NodeRange { node_id, range });
                return;
            }
        }

        match self
            .type_ranges
            .binary_search_by_key(&node_id, |entry| entry.node_id)
        {
            Ok(index) => self.type_ranges[index].range = range,
            Err(index) => self.type_ranges.insert(index, NodeRange { node_id, range }),
        }
    }

    /// Return one sparse side range by node span key.
    fn get_sparse_side_range(&self, key: NodeSpanKey) -> Option<ByteRange> {
        self.side_ranges
            .binary_search_by_key(&key, |entry| entry.key)
            .ok()
            .map(|index| self.side_ranges[index].range)
    }

    /// Set one sparse side range by node span key.
    fn set_sparse_side_range(&mut self, key: NodeSpanKey, range: ByteRange) {
        if let Some(last) = self.side_ranges.last_mut() {
            if last.key == key {
                last.range = range;
                return;
            }
            if last.key < key {
                self.side_ranges.push(NodeRangeEntry { key, range });
                return;
            }
        }

        match self
            .side_ranges
            .binary_search_by_key(&key, |entry| entry.key)
        {
            Ok(index) => self.side_ranges[index].range = range,
            Err(index) => self
                .side_ranges
                .insert(index, NodeRangeEntry { key, range }),
        }
    }

    /// Expand one node range with its source file.
    #[inline]
    fn expand_range(&self, node_id: u32, range: ByteRange) -> Span {
        Span::new(self.file_for_node(node_id), range.start, range.end)
    }

    /// Begin or continue the source file run for the next node.
    #[inline]
    fn begin_file_run(&mut self, file: FileId) {
        let node_id = self.enclosing_ranges.len() as u32;
        // replace a pending run that has not received any nodes
        if let Some(last) = self.file_runs.last_mut()
            && last.first_node_id == node_id
        {
            last.file = file;
        }
        // begin a run when the source file changes
        else if self.file_runs.last().map(|run| run.file) != Some(file) {
            self.file_runs.push(SourceFileRun {
                first_node_id: node_id,
                file,
            });
        }
    }

    /// Return the source file for one node id.
    #[inline]
    fn file_for_node(&self, node_id: u32) -> FileId {
        let index = self
            .file_runs
            .partition_point(|run| run.first_node_id <= node_id);

        if index == 0 {
            FileId::new(0)
        } else {
            self.file_runs[index - 1].file
        }
    }

    /// Update the source file for one existing node id.
    fn set_file_for_node(&mut self, node_id: u32, file: FileId) {
        if self.file_for_node(node_id) == file {
            return;
        }

        let mut file_runs = Vec::with_capacity(self.file_runs.len() + 2);
        let node_count = self.enclosing_ranges.len() as u32;

        for index in 0..self.file_runs.len() {
            let run = self.file_runs[index];
            let next_node_id = self
                .file_runs
                .get(index + 1)
                .map(|run| run.first_node_id)
                .unwrap_or(node_count);

            if node_id < run.first_node_id || node_id >= next_node_id {
                Self::push_file_run(&mut file_runs, run);
                continue;
            }

            if run.first_node_id < node_id {
                Self::push_file_run(&mut file_runs, run);
            }

            Self::push_file_run(
                &mut file_runs,
                SourceFileRun {
                    first_node_id: node_id,
                    file,
                },
            );

            if node_id + 1 < next_node_id {
                Self::push_file_run(
                    &mut file_runs,
                    SourceFileRun {
                        first_node_id: node_id + 1,
                        file: run.file,
                    },
                );
            }
        }

        self.file_runs = file_runs;
    }

    /// Push one file run and merge adjacent equal files.
    fn push_file_run(file_runs: &mut Vec<SourceFileRun>, run: SourceFileRun) {
        if file_runs.last().map(|last| last.file) == Some(run.file) {
            return;
        }

        file_runs.push(run);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_spans_keeps_file_identity() {
        let first_file = FileId::new(1);
        let second_file = FileId::new(2);
        let mut source_index = SourceIndex::new();

        source_index.append(Span::new(first_file, 0, 10));
        source_index.append(Span::new(first_file, 10, 20));
        source_index.append(Span::new(second_file, 0, 8));

        assert_eq!(source_index.get(0), Span::new(first_file, 0, 10));
        assert_eq!(source_index.get(1), Span::new(first_file, 10, 20));
        assert_eq!(source_index.get(2), Span::new(second_file, 0, 8));
        assert_eq!(source_index.file_runs.len(), 2);
    }

    /// Parsed byte ranges inherit each active source file.
    #[test]
    fn test_append_parsed_ranges_keeps_file_identity() {
        let first_file = FileId::new(1);
        let second_file = FileId::new(2);
        let mut source_index = SourceIndex::new();

        source_index.begin_source_file(first_file);
        source_index.append_parsed(ByteRange { start: 0, end: 10 });
        source_index.append_parsed(ByteRange { start: 10, end: 20 });
        source_index.begin_source_file(second_file);
        source_index.append_parsed(ByteRange { start: 0, end: 8 });

        assert_eq!(source_index.get(0), Span::new(first_file, 0, 10));
        assert_eq!(source_index.get_range(1), ByteRange { start: 10, end: 20 });
        assert_eq!(source_index.get(2), Span::new(second_file, 0, 8));
        assert_eq!(source_index.file_runs.len(), 2);
    }

    /// Empty parsed files do not retain source file runs without nodes.
    #[test]
    fn test_begin_source_file_replaces_empty_file_run() {
        let first_file = FileId::new(1);
        let second_file = FileId::new(2);
        let mut source_index = SourceIndex::new();

        source_index.begin_source_file(first_file);
        source_index.begin_source_file(second_file);
        source_index.append_parsed(ByteRange { start: 0, end: 8 });

        assert_eq!(source_index.get(0), Span::new(second_file, 0, 8));
        assert_eq!(source_index.file_runs.len(), 1);
    }

    /// Side byte ranges expand under the owning node's source file.
    #[test]
    fn test_set_side_range_keeps_node_file_identity() {
        let file = FileId::new(7);
        let mut source_index = SourceIndex::new();

        source_index.begin_source_file(file);
        source_index.append_parsed(ByteRange { start: 10, end: 30 });
        source_index.set_side_range(
            0,
            NodeSpanType::Region(NodeSpanRegion::Opening),
            ByteRange { start: 10, end: 14 },
        );

        assert_eq!(
            source_index.get_side(0, NodeSpanType::Region(NodeSpanRegion::Opening)),
            Some(Span::new(file, 10, 14))
        );
        assert_eq!(
            source_index.get_side_range(0, NodeSpanType::Region(NodeSpanRegion::Opening)),
            Some(ByteRange { start: 10, end: 14 })
        );
    }

    #[test]
    fn test_prune_spans_drops_file_runs() {
        let first_file = FileId::new(1);
        let second_file = FileId::new(2);
        let mut source_index = SourceIndex::new();

        source_index.append(Span::new(first_file, 0, 10));
        source_index.append(Span::new(second_file, 0, 8));
        source_index.prune_from(1, 1);
        source_index.append(Span::new(first_file, 10, 20));

        assert_eq!(source_index.get(0), Span::new(first_file, 0, 10));
        assert_eq!(source_index.get(1), Span::new(first_file, 10, 20));
        assert_eq!(source_index.file_runs.len(), 1);
    }

    #[test]
    fn test_prune_spans_truncates_sparse_indexes() {
        let file = FileId::new(1);
        let mut source_index = SourceIndex::new();
        source_index.append(Span::new(file, 0, 10));
        source_index.append(Span::new(file, 10, 20));
        source_index.append(Span::new(file, 20, 30));

        source_index.set_main(0, Span::new(file, 1, 9));
        source_index.set_main(2, Span::new(file, 21, 29));
        source_index.set_side(
            1,
            NodeSpanType::Region(NodeSpanRegion::Type),
            Span::new(file, 11, 19),
        );
        source_index.set_side(
            2,
            NodeSpanType::Region(NodeSpanRegion::Type),
            Span::new(file, 21, 29),
        );
        source_index.set_side(
            0,
            NodeSpanType::Region(NodeSpanRegion::Opening),
            Span::new(file, 0, 2),
        );
        source_index.set_side(
            2,
            NodeSpanType::Region(NodeSpanRegion::Opening),
            Span::new(file, 20, 22),
        );

        source_index.prune_from(2, 2);

        assert_eq!(source_index.get_main(0), Some(Span::new(file, 1, 9)));
        assert_eq!(source_index.get_main(2), None);
        assert_eq!(
            source_index.get_side(1, NodeSpanType::Region(NodeSpanRegion::Type)),
            Some(Span::new(file, 11, 19))
        );
        assert_eq!(
            source_index.get_side(2, NodeSpanType::Region(NodeSpanRegion::Type)),
            None
        );
        assert_eq!(
            source_index.get_side(0, NodeSpanType::Region(NodeSpanRegion::Opening)),
            Some(Span::new(file, 0, 2))
        );
        assert_eq!(
            source_index.get_side(2, NodeSpanType::Region(NodeSpanRegion::Opening)),
            None
        );
    }

    #[test]
    fn test_roundtrip_serialized_ranges_keeps_file_identity() {
        let first_file = FileId::new(1);
        let second_file = FileId::new(2);
        let mut source_index = SourceIndex::new();

        source_index.append(Span::new(first_file, 0, 10));
        source_index.append(Span::new(second_file, 0, 8));
        source_index.set_main(1, Span::new(second_file, 1, 4));

        let json = serde_json::to_string(&source_index).unwrap();
        let source_index: SourceIndex = serde_json::from_str(&json).unwrap();

        assert_eq!(source_index.get(0), Span::new(first_file, 0, 10));
        assert_eq!(source_index.get(1), Span::new(second_file, 0, 8));
        assert_eq!(source_index.get_main(1), Some(Span::new(second_file, 1, 4)));
    }

    #[test]
    fn test_position_lookup_keeps_file_identity() {
        let first_file = FileId::new(1);
        let second_file = FileId::new(2);
        let mut source_index = SourceIndex::new();

        source_index.append(Span::new(first_file, 0, 100));
        source_index.append(Span::new(first_file, 20, 40));
        source_index.append(Span::new(second_file, 0, 100));
        source_index.append(Span::new(second_file, 20, 40));

        let first_enclosing = source_index.get_enclosing_spans(first_file, 25, 25);
        let second_enclosing = source_index.get_enclosing_spans(second_file, 25, 25);

        assert_eq!(first_enclosing.len(), 2);
        assert_eq!(second_enclosing.len(), 2);
        assert!(
            first_enclosing
                .iter()
                .all(|span| span.span.file == first_file)
        );
        assert!(
            second_enclosing
                .iter()
                .all(|span| span.span.file == second_file)
        );
        assert!(first_enclosing.iter().any(|span| span.source_id == 1));
        assert!(second_enclosing.iter().any(|span| span.source_id == 3));
    }

    #[test]
    fn test_set_span_invalidates_position_lookup_when_file_changes() {
        let first_file = FileId::new(1);
        let second_file = FileId::new(2);
        let mut source_index = SourceIndex::new();

        source_index.append(Span::new(first_file, 0, 10));
        assert_eq!(source_index.get_enclosing_spans(first_file, 5, 5).len(), 1);

        source_index.set(0, Span::new(second_file, 0, 10));

        assert!(
            source_index
                .get_enclosing_spans(first_file, 5, 5)
                .is_empty()
        );
        assert_eq!(source_index.get_enclosing_spans(second_file, 5, 5).len(), 1);
    }
}
