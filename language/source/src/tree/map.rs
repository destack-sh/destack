use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use super::interval::IntervalTree;
use crate::{FileId, Span};

#[inline]
fn empty_span() -> Span {
    Span::empty(crate::FileId(0))
}

const MAIN_SPAN_FLAG: u8 = 1 << 0;
const TYPE_SPAN_FLAG: u8 = 1 << 1;

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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeSpanBoundary {
    /// The leading owned prefix span of a node.
    Leading,
    /// The leading operator span of a node.
    LeadingOperator,
    /// The trailing owned suffix span of a node.
    Trailing,
}

/// A named source region within one node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeSpanList {
    /// One generic ordered source segment.
    Segment,
    /// One source entry.
    Entry,
}

/// One typed source part keyed by source node id and span kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourcePartKey {
    /// The source node id that owns this source part.
    pub source_id: u32,
    /// The span kind within that source node.
    pub span_type: NodeSpanType,
}

impl SourcePartKey {
    /// Build one typed source part key.
    pub fn new(source_id: u32, span_type: NodeSpanType) -> Self {
        Self {
            source_id,
            span_type,
        }
    }
}

/// Side index of spans into a Tree.
#[derive(Debug)]
pub struct NodeSourceMap {
    /// Enclosing spans of all nodes, indexed by global node id.
    enclosing_spans: Vec<Span>,
    /// Main spans, indexed by global node id.
    main_spans: Vec<Span>,
    /// Type spans, indexed by global node id.
    type_spans: Vec<Span>,
    /// Presence flags for main and type spans.
    side_span_flags: Vec<u8>,
    /// Extra side spans for non-main/type spans (sparse).
    side_spans: FxHashMap<SourcePartKey, Span>,
    /// Interval tree for O(log n + k) enclosing span queries.
    /// Built lazily on first lookup and invalidated on enclosing span mutations.
    interval_tree: RwLock<Option<IntervalTree>>,
    /// Whether the interval tree cache is currently built.
    interval_tree_ready: AtomicBool,
}

impl Clone for NodeSourceMap {
    fn clone(&self) -> Self {
        Self {
            enclosing_spans: self.enclosing_spans.clone(),
            main_spans: self.main_spans.clone(),
            type_spans: self.type_spans.clone(),
            side_span_flags: self.side_span_flags.clone(),
            side_spans: self.side_spans.clone(),
            interval_tree: RwLock::new(None),
            interval_tree_ready: AtomicBool::new(false),
        }
    }
}

// serde representation for NodeSourceMap
#[derive(Serialize, Deserialize)]
struct NodeSourceMapData {
    enclosing_spans: Vec<Span>,
    #[serde(default)]
    main_spans: Vec<Option<Span>>,
    #[serde(default)]
    type_spans: Vec<Option<Span>>,
    #[serde(default)]
    side_spans: FxHashMap<SourcePartKey, Span>,
}

// serde view for NodeSourceMap
#[derive(Serialize)]
struct NodeSourceMapRef<'a> {
    enclosing_spans: &'a [Span],
    main_spans: Vec<Option<Span>>,
    type_spans: Vec<Option<Span>>,
    side_spans: &'a FxHashMap<SourcePartKey, Span>,
}

impl Serialize for NodeSourceMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut main_spans = Vec::with_capacity(self.enclosing_spans.len());
        let mut type_spans = Vec::with_capacity(self.enclosing_spans.len());
        for index in 0..self.enclosing_spans.len() {
            let flags = self.side_span_flags.get(index).copied().unwrap_or(0);
            let main_span = ((flags & MAIN_SPAN_FLAG) != 0).then(|| self.main_spans[index]);
            main_spans.push(main_span);

            let type_span = ((flags & TYPE_SPAN_FLAG) != 0).then(|| self.type_spans[index]);
            type_spans.push(type_span);
        }

        let data = NodeSourceMapRef {
            enclosing_spans: &self.enclosing_spans,
            main_spans,
            type_spans,
            side_spans: &self.side_spans,
        };
        data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NodeSourceMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = NodeSourceMapData::deserialize(deserializer)?;
        let enclosing_len = data.enclosing_spans.len();
        let mut main_spans = Vec::with_capacity(enclosing_len);
        for index in 0..enclosing_len {
            if let Some(span) = data.main_spans.get(index).copied().flatten() {
                main_spans.push(span);
            } else {
                main_spans.push(empty_span());
            }
        }

        let mut type_spans = Vec::with_capacity(enclosing_len);
        let mut side_span_flags = Vec::with_capacity(enclosing_len);
        for index in 0..enclosing_len {
            let mut flags = 0;
            if data.main_spans.get(index).copied().flatten().is_some() {
                flags |= MAIN_SPAN_FLAG;
            }
            if let Some(span) = data.type_spans.get(index).copied().flatten() {
                type_spans.push(span);
                flags |= TYPE_SPAN_FLAG;
            } else {
                type_spans.push(empty_span());
            }
            side_span_flags.push(flags);
        }

        Ok(Self {
            enclosing_spans: data.enclosing_spans,
            main_spans,
            type_spans,
            side_span_flags,
            side_spans: data.side_spans,
            interval_tree: RwLock::new(None),
            interval_tree_ready: AtomicBool::new(false),
        })
    }
}

impl Default for NodeSourceMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of finding enclosing spans at a position.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnclosingSpan {
    /// The index of the enclosing span in the map.
    pub idx: u32,
    /// The distance to the target span.
    pub distance: u32,
    /// The length of the enclosing span.
    pub length: u32,
    /// The enclosing span.
    pub span: Span,
}

impl NodeSourceMap {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            enclosing_spans: Vec::with_capacity(capacity),
            main_spans: Vec::with_capacity(capacity / 4),
            type_spans: Vec::with_capacity(capacity / 8),
            side_span_flags: Vec::with_capacity(capacity / 4),
            side_spans: FxHashMap::default(),
            interval_tree: RwLock::new(None),
            interval_tree_ready: AtomicBool::new(false),
        }
    }

    /// Invalidate cached position index after enclosing span mutations.
    #[inline]
    fn invalidate_position_index(&mut self) {
        if !self.interval_tree_ready.load(Ordering::Relaxed) {
            return;
        }

        let interval_tree = match self.interval_tree.get_mut() {
            Ok(interval_tree) => interval_tree,
            Err(error) => error.into_inner(),
        };
        *interval_tree = None;
        self.interval_tree_ready.store(false, Ordering::Relaxed);
    }

    /// Ensure the interval tree exists for enclosing span lookups.
    #[inline]
    fn ensure_position_index(&self) {
        if self.interval_tree_ready.load(Ordering::Relaxed) {
            return;
        }

        {
            let interval_tree = match self.interval_tree.read() {
                Ok(interval_tree) => interval_tree,
                Err(error) => error.into_inner(),
            };
            if interval_tree.is_some() {
                self.interval_tree_ready.store(true, Ordering::Relaxed);
                return;
            }
        }

        let tree = IntervalTree::build_from_spans(&self.enclosing_spans);
        let mut interval_tree = match self.interval_tree.write() {
            Ok(interval_tree) => interval_tree,
            Err(error) => error.into_inner(),
        };
        if interval_tree.is_none() {
            *interval_tree = Some(tree);
        }

        self.interval_tree_ready.store(true, Ordering::Relaxed);
    }

    /// Append a span to the map.
    #[inline]
    pub fn append(&mut self, span: Span) {
        self.enclosing_spans.push(span);
        self.invalidate_position_index();
    }

    /// Append a span while building a fresh tree during parsing.
    #[inline]
    pub fn append_during_parse(&mut self, span: Span) {
        self.enclosing_spans.push(span);
    }

    /// Set the span for a node.
    #[inline]
    pub fn set(&mut self, node_id: u32, span: Span) {
        let index = node_id as usize;
        let previous_span = self.enclosing_spans[index];
        self.enclosing_spans[index] = span;

        if previous_span != span {
            self.invalidate_position_index();
        }
    }

    /// Prune spans from the map (used during parse backtracking).
    #[inline]
    pub fn prune_from(&mut self, from_idx: u32) {
        self.enclosing_spans.truncate(from_idx as usize);
        self.main_spans.truncate(from_idx as usize);
        self.type_spans.truncate(from_idx as usize);
        self.side_span_flags.truncate(from_idx as usize);
        self.side_spans.retain(|key, _| key.source_id < from_idx);
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
        let index = node_id as usize;
        match span_type {
            NodeSpanType::Main => {
                if index >= self.main_spans.len() {
                    self.main_spans.resize(index + 1, empty_span());
                }
                if index >= self.side_span_flags.len() {
                    self.side_span_flags.resize(index + 1, 0);
                }
                self.main_spans[index] = span;
                self.side_span_flags[index] |= MAIN_SPAN_FLAG;
            }
            NodeSpanType::Region(NodeSpanRegion::Type) => {
                if index >= self.type_spans.len() {
                    self.type_spans.resize(index + 1, empty_span());
                }
                if index >= self.side_span_flags.len() {
                    self.side_span_flags.resize(index + 1, 0);
                }
                self.type_spans[index] = span;
                self.side_span_flags[index] |= TYPE_SPAN_FLAG;
            }
            _ => {
                self.side_spans
                    .insert(SourcePartKey::new(node_id, span_type), span);
            }
        }
    }

    /// Get a side span for a node, if it has one.
    #[inline]
    pub fn get_side(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        let index = node_id as usize;
        match span_type {
            NodeSpanType::Main => self
                .side_span_flags
                .get(index)
                .copied()
                .filter(|flags| (flags & MAIN_SPAN_FLAG) != 0)
                .map(|_| self.main_spans[index]),
            NodeSpanType::Region(NodeSpanRegion::Type) => self
                .side_span_flags
                .get(index)
                .copied()
                .filter(|flags| (flags & TYPE_SPAN_FLAG) != 0)
                .map(|_| self.type_spans[index]),
            _ => self
                .side_spans
                .get(&SourcePartKey::new(node_id, span_type))
                .copied(),
        }
    }

    /// Find the innermost side span of one kind that fully contains a span.
    pub fn find_innermost_side_owner(
        &self,
        span: Span,
        span_type: NodeSpanType,
    ) -> Option<SourcePartKey> {
        self.side_spans
            .iter()
            .filter_map(|(key, part_span)| {
                (key.span_type == span_type
                    && part_span.file == span.file
                    && part_span.start <= span.start
                    && part_span.end >= span.end)
                    .then_some((*key, *part_span))
            })
            .min_by_key(|(_, part_span)| part_span.end - part_span.start)
            .map(|(key, _)| key)
    }

    /// Find the innermost non-enclosing source part that fully contains a span.
    pub fn find_innermost_part_owner(&self, span: Span) -> Option<SourcePartKey> {
        let mut best_owner = None;
        let mut best_length = u32::MAX;

        // dense main and type spans
        for (index, flags) in self.side_span_flags.iter().copied().enumerate() {
            let source_id = index as u32;

            if (flags & MAIN_SPAN_FLAG) != 0 {
                let part_span = self.main_spans[index];
                if part_span.file == span.file
                    && part_span.start <= span.start
                    && part_span.end >= span.end
                {
                    let part_length = part_span.end - part_span.start;
                    if part_length < best_length {
                        best_owner = Some(SourcePartKey::new(source_id, NodeSpanType::Main));
                        best_length = part_length;
                    }
                }
            }

            if (flags & TYPE_SPAN_FLAG) != 0 {
                let part_span = self.type_spans[index];
                if part_span.file == span.file
                    && part_span.start <= span.start
                    && part_span.end >= span.end
                {
                    let part_length = part_span.end - part_span.start;
                    if part_length < best_length {
                        best_owner = Some(SourcePartKey::new(
                            source_id,
                            NodeSpanType::Region(NodeSpanRegion::Type),
                        ));
                        best_length = part_length;
                    }
                }
            }
        }

        // sparse side spans
        for (key, part_span) in &self.side_spans {
            if part_span.file != span.file
                || part_span.start > span.start
                || part_span.end < span.end
            {
                continue;
            }

            let part_length = part_span.end - part_span.start;
            if part_length < best_length {
                best_owner = Some(*key);
                best_length = part_length;
            }
        }

        best_owner
    }

    /// Find the innermost enclosing owner that fully contains a span.
    pub fn find_innermost_enclosing_owner(&self, span: Span) -> Option<SourcePartKey> {
        let end_inclusive = span.end.saturating_sub(1);

        self.get_enclosing_spans(span.start, end_inclusive)
            .into_iter()
            .filter(|enclosing| {
                enclosing.span.file == span.file
                    && enclosing.span.start <= span.start
                    && enclosing.span.end >= span.end
            })
            .min_by_key(|enclosing| enclosing.length)
            .map(|enclosing| SourcePartKey::new(enclosing.idx, NodeSpanType::Enclosing))
    }

    /// Find the nearest enclosing owner that begins after a position.
    pub fn find_nearest_enclosing_owner_after(
        &self,
        file: FileId,
        position: u32,
    ) -> Option<SourcePartKey> {
        self.enclosing_spans
            .iter()
            .enumerate()
            .filter(|(_, span)| span.file == file && span.start >= position)
            .min_by_key(|(_, span)| (span.start - position, span.end - span.start))
            .map(|(index, _)| SourcePartKey::new(index as u32, NodeSpanType::Enclosing))
    }

    /// Find the nearest enclosing owner that ends before a position.
    pub fn find_nearest_enclosing_owner_before(
        &self,
        file: FileId,
        position: u32,
    ) -> Option<SourcePartKey> {
        self.enclosing_spans
            .iter()
            .enumerate()
            .filter(|(_, span)| span.file == file && span.end <= position)
            .min_by_key(|(_, span)| (position - span.end, span.end - span.start))
            .map(|(index, _)| SourcePartKey::new(index as u32, NodeSpanType::Enclosing))
    }

    /// Get a side span or the enclosing span if no side span is set.
    #[inline]
    pub fn get_side_or_enclosing(&self, node_id: u32, span_type: NodeSpanType) -> Span {
        self.get_side(node_id, span_type)
            .unwrap_or_else(|| self.get(node_id))
    }

    /// Get a side span or a main span or a enclosing span if no side or main span is set.
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
        self.enclosing_spans[node_id as usize]
    }

    /// Get the span for a node by its id when it is present.
    #[inline]
    pub fn try_get(&self, node_id: u32) -> Option<Span> {
        self.enclosing_spans.get(node_id as usize).copied()
    }

    /// Get all enclosing spans containing the given range.
    ///
    /// Uses interval tree for O(log n + k) lookup where k is the number of enclosing spans.
    pub fn get_enclosing_spans(&self, start: u32, end_inclusive: u32) -> Vec<EnclosingSpan> {
        self.ensure_position_index();

        let Some(first_span) = self.enclosing_spans.first() else {
            return Vec::new();
        };
        let file_id = first_span.file;
        let interval_tree = match self.interval_tree.read() {
            Ok(interval_tree) => interval_tree,
            Err(error) => error.into_inner(),
        };
        let tree = interval_tree
            .as_ref()
            .expect("position index must exist after ensure_position_index");

        tree.query_containing(start, end_inclusive)
            .into_iter()
            .map(|(span_start, span_end, node_id, length)| {
                let distance = start.abs_diff(span_start) + span_end.abs_diff(end_inclusive);
                EnclosingSpan {
                    idx: node_id,
                    distance,
                    length,
                    span: Span {
                        file: file_id,
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
        start: u32,
        end_inclusive: u32,
        mut visit: impl FnMut(EnclosingSpan),
    ) {
        self.ensure_position_index();

        let Some(first_span) = self.enclosing_spans.first() else {
            return;
        };
        let file_id = first_span.file;
        let interval_tree = match self.interval_tree.read() {
            Ok(interval_tree) => interval_tree,
            Err(error) => error.into_inner(),
        };
        let tree = interval_tree
            .as_ref()
            .expect("position index must exist after ensure_position_index");

        tree.visit_containing(
            start,
            end_inclusive,
            |span_start, span_end, node_id, length| {
                let distance = start.abs_diff(span_start) + span_end.abs_diff(end_inclusive);
                visit(EnclosingSpan {
                    idx: node_id,
                    distance,
                    length,
                    span: Span {
                        file: file_id,
                        start: span_start,
                        end: span_end,
                    },
                });
            },
        );
    }

    /// Rebind all spans in the map to one file id.
    pub fn rebind_file(&mut self, file: crate::FileId) {
        // enclosing spans
        for span in &mut self.enclosing_spans {
            span.file = file;
        }

        // main spans
        for (index, span) in self.main_spans.iter_mut().enumerate() {
            let Some(flags) = self.side_span_flags.get(index) else {
                continue;
            };
            if (flags & MAIN_SPAN_FLAG) != 0 {
                span.file = file;
            }
        }

        // type spans
        for (index, span) in self.type_spans.iter_mut().enumerate() {
            let Some(flags) = self.side_span_flags.get(index) else {
                continue;
            };
            if (flags & TYPE_SPAN_FLAG) != 0 {
                span.file = file;
            }
        }

        // extra side spans
        for span in self.side_spans.values_mut() {
            span.file = file;
        }

        self.invalidate_position_index();
    }
}
