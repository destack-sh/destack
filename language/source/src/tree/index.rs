use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::interval::IntervalTree;
use crate::{FileId, Span};

/// The type of node search to perform.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeSearchMode {
    /// Search for the biggest outermost node that matches.
    BiggestOutermost,
    /// Search for the smallest outermost node that matches.
    SmallestOutermost,
    /// Search for the smallest innermost node that matches.
    SmallestInnermost,
}

/// The type of span for a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NodeSpanBoundary {
    /// The leading owned prefix span of a node.
    Leading,
    /// The leading operator span of a node.
    LeadingOperator,
    /// The trailing owned suffix span of a node.
    Trailing,
}

/// A named source region within one node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    /// A transparent syntactic wrapper around a node.
    Wrapper,
}

/// An indexed source list within one node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NodeSpanList {
    /// One generic ordered source segment.
    Segment,
    /// One source entry.
    Entry,
}

/// One typed node span keyed by source node id and span kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// A source range without repeated file identity.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct PackedSpan {
    /// The start position in bytes.
    start: u32,
    /// The end position in bytes.
    end: u32,
}

impl PackedSpan {
    /// Pack one full span.
    #[inline]
    fn from_span(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }

    /// Expand this range into a full span.
    #[inline]
    fn to_span(self, file: FileId) -> Span {
        Span {
            file,
            start: self.start,
            end: self.end,
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

/// One packed span keyed by source node id.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct NodePackedSpan {
    /// The source node id.
    node_id: u32,
    /// The packed source range.
    span: PackedSpan,
}

/// One sparse side span.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct NodeSpanEntry {
    /// The node and span kind.
    key: NodeSpanKey,
    /// The packed source range.
    span: PackedSpan,
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

        for (index, span) in source_index.enclosing_spans.iter().copied().enumerate() {
            let node_id = index as u32;
            let file = source_index.file_for_node(node_id);
            let interval = (span.start, span.end, node_id);

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

/// Source range and anchor index for tree nodes.
#[derive(Debug)]
pub struct SourceIndex {
    /// Source file runs for packed node spans.
    file_runs: Vec<SourceFileRun>,
    /// Enclosing spans of all nodes, indexed by global node id.
    enclosing_spans: Vec<PackedSpan>,
    /// Main spans keyed by global node id.
    main_spans: Vec<NodePackedSpan>,
    /// Type spans keyed by global node id.
    type_spans: Vec<NodePackedSpan>,
    /// Extra side spans for non-main/type spans.
    side_spans: Vec<NodeSpanEntry>,
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
            enclosing_spans: self.enclosing_spans.clone(),
            main_spans: self.main_spans.clone(),
            type_spans: self.type_spans.clone(),
            side_spans: self.side_spans.clone(),
            position_index: RwLock::new(None),
            position_index_ready: AtomicBool::new(false),
        }
    }
}

/// Serialized source index shape.
#[derive(Serialize, Deserialize)]
struct SourceIndexData {
    enclosing_spans: Vec<Span>,
    #[serde(default)]
    main_spans: Vec<Option<Span>>,
    #[serde(default)]
    type_spans: Vec<Option<Span>>,
    #[serde(default)]
    side_spans: FxHashMap<NodeSpanKey, Span>,
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
        let enclosing_spans = self.full_enclosing_spans();
        let mut main_spans = Vec::with_capacity(self.enclosing_spans.len());
        let mut type_spans = Vec::with_capacity(self.enclosing_spans.len());
        for index in 0..self.enclosing_spans.len() {
            main_spans.push(
                self.get_main_span(index as u32)
                    .map(|span| span.to_span(self.file_for_node(index as u32))),
            );
            type_spans.push(
                self.get_type_span(index as u32)
                    .map(|span| span.to_span(self.file_for_node(index as u32))),
            );
        }
        let side_spans = self.full_side_spans();

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
        let (file_runs, enclosing_spans) = Self::pack_enclosing_spans(data.enclosing_spans);
        let mut main_spans = Vec::new();
        for index in 0..enclosing_len {
            if let Some(span) = data.main_spans.get(index).copied().flatten() {
                main_spans.push(NodePackedSpan {
                    node_id: index as u32,
                    span: PackedSpan::from_span(span),
                });
            }
        }

        let mut type_spans = Vec::new();
        for index in 0..enclosing_len {
            if let Some(span) = data.type_spans.get(index).copied().flatten() {
                type_spans.push(NodePackedSpan {
                    node_id: index as u32,
                    span: PackedSpan::from_span(span),
                });
            }
        }
        let mut side_spans: Vec<NodeSpanEntry> = data
            .side_spans
            .into_iter()
            .map(|(key, span)| NodeSpanEntry {
                key,
                span: PackedSpan::from_span(span),
            })
            .collect();
        side_spans.sort_unstable_by_key(|entry| entry.key);

        Ok(Self {
            file_runs,
            enclosing_spans,
            main_spans,
            type_spans,
            side_spans,
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
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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
            enclosing_spans: Vec::with_capacity(capacity),
            main_spans: Vec::new(),
            type_spans: Vec::new(),
            side_spans: Vec::new(),
            position_index: RwLock::new(None),
            position_index_ready: AtomicBool::new(false),
        }
    }

    /// Return whether this index has an enclosing span for a node id.
    #[inline]
    pub fn contains_node(&self, node_id: u32) -> bool {
        (node_id as usize) < self.enclosing_spans.len()
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
        self.push_file(span.file);
        self.enclosing_spans.push(PackedSpan::from_span(span));
        self.invalidate_position_index();
    }

    /// Append a span while building a fresh tree during parsing.
    #[inline]
    pub fn append_during_parse(&mut self, span: Span) {
        self.push_file(span.file);
        self.enclosing_spans.push(PackedSpan::from_span(span));
    }

    /// Set the span for a node.
    #[inline]
    pub fn set(&mut self, node_id: u32, span: Span) {
        let index = node_id as usize;
        let previous_span = self.enclosing_spans[index];
        let previous_file = self.file_for_node(node_id);
        let file = span.file;
        let span = PackedSpan::from_span(span);
        self.enclosing_spans[index] = span;
        self.set_file_for_node(node_id, file);

        if previous_span != span || previous_file != file {
            self.invalidate_position_index();
        }
    }

    /// Prune spans from the index after one mark.
    #[inline]
    pub fn prune_from(&mut self, retained_node_count: usize, first_pruned_node_id: u32) {
        self.enclosing_spans.truncate(retained_node_count);
        self.file_runs
            .retain(|run| (run.first_node_id as usize) < retained_node_count);
        self.main_spans
            .retain(|entry| (entry.node_id as usize) < retained_node_count);
        self.type_spans
            .retain(|entry| (entry.node_id as usize) < retained_node_count);
        self.side_spans
            .retain(|entry| entry.key.source_id < first_pruned_node_id);
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
                self.set_main_span(node_id, PackedSpan::from_span(span));
            }
            NodeSpanType::Region(NodeSpanRegion::Type) => {
                self.set_type_span(node_id, PackedSpan::from_span(span));
            }
            _ => {
                let key = NodeSpanKey::new(node_id, span_type);
                self.set_sparse_side_span(key, PackedSpan::from_span(span));
            }
        }
    }

    /// Get a side span for a node, if it has one.
    #[inline]
    pub fn get_side(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        match span_type {
            NodeSpanType::Main => self
                .get_main_span(node_id)
                .map(|span| span.to_span(self.file_for_node(node_id))),
            NodeSpanType::Region(NodeSpanRegion::Type) => self
                .get_type_span(node_id)
                .map(|span| span.to_span(self.file_for_node(node_id))),
            _ => self
                .get_sparse_side_span(NodeSpanKey::new(node_id, span_type))
                .map(|span| span.to_span(self.file_for_node(node_id))),
        }
    }

    /// Find the innermost side span of one kind that fully contains a span.
    pub fn find_innermost_side_owner(
        &self,
        span: Span,
        span_type: NodeSpanType,
    ) -> Option<NodeSpanKey> {
        self.side_spans
            .iter()
            .filter_map(|entry| {
                (entry.key.span_type == span_type
                    && self.file_for_node(entry.key.source_id) == span.file
                    && entry.span.start <= span.start
                    && entry.span.end >= span.end)
                    .then_some((entry.key, entry.span))
            })
            .min_by_key(|(_, node_span)| node_span.end - node_span.start)
            .map(|(key, _)| key)
    }

    /// Find the innermost non-enclosing node span that fully contains a span.
    pub fn find_innermost_node_span_owner(&self, span: Span) -> Option<NodeSpanKey> {
        let mut best_owner = None;
        let mut best_length = u32::MAX;

        // main spans
        for entry in &self.main_spans {
            if self.file_for_node(entry.node_id) == span.file
                && entry.span.start <= span.start
                && entry.span.end >= span.end
            {
                let span_length = entry.span.end - entry.span.start;
                if span_length < best_length {
                    best_owner = Some(NodeSpanKey::new(entry.node_id, NodeSpanType::Main));
                    best_length = span_length;
                }
            }
        }

        // type spans
        for entry in &self.type_spans {
            if self.file_for_node(entry.node_id) == span.file
                && entry.span.start <= span.start
                && entry.span.end >= span.end
            {
                let span_length = entry.span.end - entry.span.start;
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
        for entry in &self.side_spans {
            if self.file_for_node(entry.key.source_id) != span.file
                || entry.span.start > span.start
                || entry.span.end < span.end
            {
                continue;
            }

            let span_length = entry.span.end - entry.span.start;
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
        self.enclosing_spans
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
        self.enclosing_spans
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
        self.enclosing_spans[node_id as usize].to_span(self.file_for_node(node_id))
    }

    /// Get the span for a node by its id when it is present.
    #[inline]
    pub fn try_get(&self, node_id: u32) -> Option<Span> {
        self.enclosing_spans
            .get(node_id as usize)
            .copied()
            .map(|span| span.to_span(self.file_for_node(node_id)))
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

        if self.enclosing_spans.is_empty() {
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
            .map(|(span_start, span_end, node_id, length)| {
                let distance = start.abs_diff(span_start) + span_end.abs_diff(end_inclusive);
                EnclosingSpan {
                    source_id: node_id,
                    distance,
                    length,
                    span: Span {
                        file,
                        start: span_start,
                        end: span_end,
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

        if self.enclosing_spans.is_empty() {
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
            |span_start, span_end, node_id, length| {
                let distance = start.abs_diff(span_start) + span_end.abs_diff(end_inclusive);
                visit(EnclosingSpan {
                    source_id: node_id,
                    distance,
                    length,
                    span: Span {
                        file,
                        start: span_start,
                        end: span_end,
                    },
                });
            },
        );
    }

    /// Rebind all spans in the index to one file id.
    pub fn rebind_file(&mut self, file: crate::FileId) {
        if self.enclosing_spans.is_empty() {
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

    /// Pack enclosing spans and their source files.
    fn pack_enclosing_spans(spans: Vec<Span>) -> (Vec<SourceFileRun>, Vec<PackedSpan>) {
        let mut file_runs = Vec::new();
        let mut packed_spans = Vec::with_capacity(spans.len());

        for span in spans {
            let node_id = packed_spans.len() as u32;
            if file_runs.last().map(|run: &SourceFileRun| run.file) != Some(span.file) {
                file_runs.push(SourceFileRun {
                    first_node_id: node_id,
                    file: span.file,
                });
            }

            packed_spans.push(PackedSpan::from_span(span));
        }

        (file_runs, packed_spans)
    }

    /// Return full enclosing spans for serialization.
    fn full_enclosing_spans(&self) -> Vec<Span> {
        self.enclosing_spans
            .iter()
            .copied()
            .enumerate()
            .map(|(index, span)| span.to_span(self.file_for_node(index as u32)))
            .collect()
    }

    /// Return full sparse side spans for serialization.
    fn full_side_spans(&self) -> FxHashMap<NodeSpanKey, Span> {
        self.side_spans
            .iter()
            .map(|entry| {
                (
                    entry.key,
                    entry.span.to_span(self.file_for_node(entry.key.source_id)),
                )
            })
            .collect()
    }

    /// Return one type span by source node id.
    fn get_type_span(&self, node_id: u32) -> Option<PackedSpan> {
        self.type_spans
            .binary_search_by_key(&node_id, |entry| entry.node_id)
            .ok()
            .map(|index| self.type_spans[index].span)
    }

    /// Return one main span by source node id.
    fn get_main_span(&self, node_id: u32) -> Option<PackedSpan> {
        self.main_spans
            .binary_search_by_key(&node_id, |entry| entry.node_id)
            .ok()
            .map(|index| self.main_spans[index].span)
    }

    /// Set one main span by source node id.
    fn set_main_span(&mut self, node_id: u32, span: PackedSpan) {
        if let Some(last) = self.main_spans.last_mut() {
            if last.node_id == node_id {
                last.span = span;
                return;
            }
            if last.node_id < node_id {
                self.main_spans.push(NodePackedSpan { node_id, span });
                return;
            }
        }

        match self
            .main_spans
            .binary_search_by_key(&node_id, |entry| entry.node_id)
        {
            Ok(index) => self.main_spans[index].span = span,
            Err(index) => self
                .main_spans
                .insert(index, NodePackedSpan { node_id, span }),
        }
    }

    /// Set one type span by source node id.
    fn set_type_span(&mut self, node_id: u32, span: PackedSpan) {
        if let Some(last) = self.type_spans.last_mut() {
            if last.node_id == node_id {
                last.span = span;
                return;
            }
            if last.node_id < node_id {
                self.type_spans.push(NodePackedSpan { node_id, span });
                return;
            }
        }

        match self
            .type_spans
            .binary_search_by_key(&node_id, |entry| entry.node_id)
        {
            Ok(index) => self.type_spans[index].span = span,
            Err(index) => self
                .type_spans
                .insert(index, NodePackedSpan { node_id, span }),
        }
    }

    /// Return one sparse side span by node span key.
    fn get_sparse_side_span(&self, key: NodeSpanKey) -> Option<PackedSpan> {
        self.side_spans
            .binary_search_by_key(&key, |entry| entry.key)
            .ok()
            .map(|index| self.side_spans[index].span)
    }

    /// Set one sparse side span by node span key.
    fn set_sparse_side_span(&mut self, key: NodeSpanKey, span: PackedSpan) {
        if let Some(last) = self.side_spans.last_mut() {
            if last.key == key {
                last.span = span;
                return;
            }
            if last.key < key {
                self.side_spans.push(NodeSpanEntry { key, span });
                return;
            }
        }

        match self
            .side_spans
            .binary_search_by_key(&key, |entry| entry.key)
        {
            Ok(index) => self.side_spans[index].span = span,
            Err(index) => self.side_spans.insert(index, NodeSpanEntry { key, span }),
        }
    }

    /// Append one file run if the appended node changes file.
    #[inline]
    fn push_file(&mut self, file: FileId) {
        let node_id = self.enclosing_spans.len() as u32;
        if self.file_runs.last().map(|run| run.file) != Some(file) {
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
        let node_count = self.enclosing_spans.len() as u32;

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
    fn test_pack_spans_keeps_file_identity() {
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
    fn test_serialize_packed_spans_as_full_spans() {
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
