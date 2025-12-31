use std::collections::HashMap;

use crate::Span;

/// The type of node search to perform.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum NodeSearchMode {
    /// Search for the biggest outermost node that matches.
    BiggestOutermost,
    /// Search for the smallest outermost node that matches.
    SmallestOutermost,
    /// Search for the smallest innermost node that matches.
    SmallestInnermost,
}

/// The type of span for a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum NodeSpanType {
    /// The enclosing span of a node.
    Enclosing,
    /// The main span of a node (usually its identifier).
    Main,
    /// The type declaration span of a node.
    Type,
}

/// Side index of spans into a NodeTree.
#[derive(Debug, Clone)]
pub struct NodeSourceMap {
    /// Enclosing spans of all nodes, indexed by global node id.
    enclosing_spans: Vec<Span>,
    /// Extra side spans for nodes that have them (sparse).
    side_spans: HashMap<(u32, NodeSpanType), Span>,
    /// Position index for fast enclosing span lookups: (start, end, node_id) sorted by start.
    /// Built via `build_position_index()` after parsing completes.
    start_index: Option<Vec<(u32, u32, u32)>>,
    /// End position index: (end, node_id) sorted by end descending.
    /// Used with start_index for O(log n + k) enclosing span queries.
    end_index: Option<Vec<(u32, u32)>>,
}

impl Default for NodeSourceMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of finding enclosing spans at a position.
#[derive(Debug, Copy, Clone, PartialEq)]
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
        Self {
            enclosing_spans: Vec::new(),
            side_spans: HashMap::new(),
            start_index: None,
            end_index: None,
        }
    }

    /// Append a span to the map.
    #[inline]
    pub fn append(&mut self, span: Span) {
        self.enclosing_spans.push(span);
    }

    /// Set the span for a node.
    #[inline]
    pub fn set(&mut self, node_id: u32, span: Span) {
        self.enclosing_spans[node_id as usize] = span;
    }

    /// Prune spans from the map (used during parse backtracking).
    #[inline]
    pub fn prune_from(&mut self, from_idx: u32) {
        self.enclosing_spans.truncate(from_idx as usize);
        self.side_spans.retain(|&(id, _), _| id < from_idx);
    }

    /// Build position indices for fast enclosing span lookups.
    /// Call this after parsing is complete.
    pub fn build_position_index(&mut self) {
        // start index: (start, end, node_id) sorted by start ascending
        let mut start_idx: Vec<(u32, u32, u32)> = self
            .enclosing_spans
            .iter()
            .enumerate()
            .map(|(i, span)| (span.start, span.end, i as u32))
            .collect();
        start_idx.sort_unstable_by_key(|(start, _, _)| *start);
        self.start_index = Some(start_idx);

        // end index: (end, node_id) sorted by end descending
        let mut end_idx: Vec<(u32, u32)> = self
            .enclosing_spans
            .iter()
            .enumerate()
            .map(|(i, span)| (span.end, i as u32))
            .collect();
        end_idx.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        self.end_index = Some(end_idx);
    }

    /// Set a side span for a node.
    #[inline]
    pub fn set_side(&mut self, node_id: u32, span_type: NodeSpanType, span: Span) {
        self.side_spans.insert((node_id, span_type), span);
    }

    /// Get a side span for a node, if it has one.
    #[inline]
    pub fn get_side(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        self.side_spans.get(&(node_id, span_type)).copied()
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
    /// Uses dual binary search for O(log n + k) lookup where k is the number of enclosing spans.
    /// Falls back to linear scan if position indices haven't been built.
    pub fn get_enclosing_spans(&self, start: u32, end_inclusive: u32) -> Vec<EnclosingSpan> {
        match (&self.start_index, &self.end_index) {
            (Some(start_idx), Some(end_idx)) => {
                self.get_enclosing_spans_indexed(start_idx, end_idx, start, end_inclusive)
            }
            _ => self.get_enclosing_spans_linear(start, end_inclusive),
        }
    }

    /// Get all enclosing spans containing the given range using dual indices.
    fn get_enclosing_spans_indexed(
        &self,
        start_index: &[(u32, u32, u32)],
        end_index: &[(u32, u32)],
        start: u32,
        end_inclusive: u32,
    ) -> Vec<EnclosingSpan> {
        // find spans where span.start <= start (candidates from start perspective)
        let start_partition = start_index.partition_point(|(s, _, _)| *s <= start);

        // find spans where span.end > end_inclusive (candidates from end perspective)
        // end_index is sorted descending, so we find first entry where end <= end_inclusive
        let end_partition = end_index.partition_point(|(e, _)| *e > end_inclusive);

        // use the smaller set as the base for intersection
        let start_count = start_partition;
        let end_count = end_partition;

        if start_count <= end_count {
            // iterate start candidates, check end condition inline
            let mut spans = Vec::new();
            for &(span_start, span_end, node_id) in &start_index[..start_partition] {
                if span_end > end_inclusive {
                    let distance = start.abs_diff(span_start) + span_end.abs_diff(end_inclusive);
                    let span = Span {
                        file: self.enclosing_spans[node_id as usize].file,
                        start: span_start,
                        end: span_end,
                    };
                    spans.push(EnclosingSpan {
                        idx: node_id,
                        distance,
                        length: span_end.saturating_sub(span_start),
                        span,
                    });
                }
            }
            spans
        } else {
            // iterate end candidates, check start condition via lookup
            let mut spans = Vec::new();
            for &(span_end, node_id) in &end_index[..end_partition] {
                let span = self.enclosing_spans[node_id as usize];
                if span.start <= start {
                    let distance = start.abs_diff(span.start) + span_end.abs_diff(end_inclusive);
                    spans.push(EnclosingSpan {
                        idx: node_id,
                        distance,
                        length: span_end.saturating_sub(span.start),
                        span,
                    });
                }
            }
            spans
        }
    }

    /// Get all enclosing spans containing the given range using a linear scan.
    fn get_enclosing_spans_linear(&self, start: u32, end_inclusive: u32) -> Vec<EnclosingSpan> {
        let mut spans = Vec::new();
        for (i, span) in self.enclosing_spans.iter().enumerate() {
            if span.contains(start) && span.contains(end_inclusive) {
                let distance = start.abs_diff(span.start) + span.end.abs_diff(end_inclusive);
                spans.push(EnclosingSpan {
                    idx: i as u32,
                    distance,
                    length: span.end.saturating_sub(span.start),
                    span: *span,
                });
            }
        }
        spans
    }
}
