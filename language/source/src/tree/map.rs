use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::RwLock;

use super::interval::IntervalTree;
use crate::Span;

#[inline]
fn empty_span() -> Span {
    Span::empty(crate::FileId(0))
}

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
    /// The type declaration span of a node.
    Type,
}

/// Side index of spans into a NodeTree.
#[derive(Debug)]
pub struct NodeSourceMap {
    /// Enclosing spans of all nodes, indexed by global node id.
    enclosing_spans: Vec<Span>,
    /// Main spans, indexed by global node id.
    main_spans: Vec<Span>,
    /// Whether a main span exists for each node.
    has_main_span: Vec<bool>,
    /// Type spans, indexed by global node id.
    type_spans: Vec<Span>,
    /// Whether a type span exists for each node.
    has_type_span: Vec<bool>,
    /// Extra side spans for non-main/type spans (sparse).
    side_spans: FxHashMap<(u32, NodeSpanType), Span>,
    /// Interval tree for O(log n + k) enclosing span queries.
    /// Built lazily on first lookup and invalidated on enclosing span mutations.
    interval_tree: RwLock<Option<IntervalTree>>,
}

impl Clone for NodeSourceMap {
    fn clone(&self) -> Self {
        Self {
            enclosing_spans: self.enclosing_spans.clone(),
            main_spans: self.main_spans.clone(),
            has_main_span: self.has_main_span.clone(),
            type_spans: self.type_spans.clone(),
            has_type_span: self.has_type_span.clone(),
            side_spans: self.side_spans.clone(),
            interval_tree: RwLock::new(None),
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
    side_spans: FxHashMap<(u32, NodeSpanType), Span>,
}

// serde view for NodeSourceMap
#[derive(Serialize)]
struct NodeSourceMapRef<'a> {
    enclosing_spans: &'a [Span],
    main_spans: Vec<Option<Span>>,
    type_spans: Vec<Option<Span>>,
    side_spans: &'a FxHashMap<(u32, NodeSpanType), Span>,
}

impl Serialize for NodeSourceMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut main_spans = Vec::with_capacity(self.enclosing_spans.len());
        let mut type_spans = Vec::with_capacity(self.enclosing_spans.len());
        for index in 0..self.enclosing_spans.len() {
            let main_span = if self.has_main_span[index] {
                Some(self.main_spans[index])
            } else {
                None
            };
            main_spans.push(main_span);

            let type_span = if self.has_type_span[index] {
                Some(self.type_spans[index])
            } else {
                None
            };
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
        let mut has_main_span = Vec::with_capacity(enclosing_len);
        for index in 0..enclosing_len {
            if let Some(span) = data.main_spans.get(index).copied().flatten() {
                main_spans.push(span);
                has_main_span.push(true);
            } else {
                main_spans.push(empty_span());
                has_main_span.push(false);
            }
        }

        let mut type_spans = Vec::with_capacity(enclosing_len);
        let mut has_type_span = Vec::with_capacity(enclosing_len);
        for index in 0..enclosing_len {
            if let Some(span) = data.type_spans.get(index).copied().flatten() {
                type_spans.push(span);
                has_type_span.push(true);
            } else {
                type_spans.push(empty_span());
                has_type_span.push(false);
            }
        }

        Ok(Self {
            enclosing_spans: data.enclosing_spans,
            main_spans,
            has_main_span,
            type_spans,
            has_type_span,
            side_spans: data.side_spans,
            interval_tree: RwLock::new(None),
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
            main_spans: Vec::with_capacity(capacity),
            has_main_span: Vec::with_capacity(capacity),
            type_spans: Vec::with_capacity(capacity),
            has_type_span: Vec::with_capacity(capacity),
            side_spans: FxHashMap::default(),
            interval_tree: RwLock::new(None),
        }
    }

    /// Invalidate cached position index after enclosing span mutations.
    #[inline]
    fn invalidate_position_index(&mut self) {
        let interval_tree = match self.interval_tree.get_mut() {
            Ok(interval_tree) => interval_tree,
            Err(error) => error.into_inner(),
        };
        *interval_tree = None;
    }

    /// Ensure the interval tree exists for enclosing span lookups.
    #[inline]
    fn ensure_position_index(&self) {
        {
            let interval_tree = match self.interval_tree.read() {
                Ok(interval_tree) => interval_tree,
                Err(error) => error.into_inner(),
            };
            if interval_tree.is_some() {
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
    }

    /// Append a span to the map.
    #[inline]
    pub fn append(&mut self, span: Span) {
        self.enclosing_spans.push(span);
        self.main_spans.push(empty_span());
        self.has_main_span.push(false);
        self.type_spans.push(empty_span());
        self.has_type_span.push(false);
        self.invalidate_position_index();
    }

    /// Set the span for a node.
    #[inline]
    pub fn set(&mut self, node_id: u32, span: Span) {
        self.enclosing_spans[node_id as usize] = span;
        self.invalidate_position_index();
    }

    /// Prune spans from the map (used during parse backtracking).
    #[inline]
    pub fn prune_from(&mut self, from_idx: u32) {
        self.enclosing_spans.truncate(from_idx as usize);
        self.main_spans.truncate(from_idx as usize);
        self.has_main_span.truncate(from_idx as usize);
        self.type_spans.truncate(from_idx as usize);
        self.has_type_span.truncate(from_idx as usize);
        self.side_spans.retain(|&(id, _), _| id < from_idx);
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
                    self.has_main_span.resize(index + 1, false);
                }
                self.main_spans[index] = span;
                self.has_main_span[index] = true;
            }
            NodeSpanType::Type => {
                if index >= self.type_spans.len() {
                    self.type_spans.resize(index + 1, empty_span());
                    self.has_type_span.resize(index + 1, false);
                }
                self.type_spans[index] = span;
                self.has_type_span[index] = true;
            }
            _ => {
                self.side_spans.insert((node_id, span_type), span);
            }
        }
    }

    /// Get a side span for a node, if it has one.
    #[inline]
    pub fn get_side(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        let index = node_id as usize;
        match span_type {
            NodeSpanType::Main => self
                .has_main_span
                .get(index)
                .copied()
                .unwrap_or(false)
                .then(|| self.main_spans[index]),
            NodeSpanType::Type => self
                .has_type_span
                .get(index)
                .copied()
                .unwrap_or(false)
                .then(|| self.type_spans[index]),
            _ => self.side_spans.get(&(node_id, span_type)).copied(),
        }
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

    /// Get all enclosing spans containing the given range.
    ///
    /// Uses interval tree for O(log n + k) lookup where k is the number of enclosing spans.
    pub fn get_enclosing_spans(&self, start: u32, end_inclusive: u32) -> Vec<EnclosingSpan> {
        self.ensure_position_index();

        let file_id = self
            .enclosing_spans
            .first()
            .map(|span| span.file)
            .unwrap_or(crate::FileId(0));
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

        let file_id = self
            .enclosing_spans
            .first()
            .map(|span| span.file)
            .unwrap_or(crate::FileId(0));
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
}
