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

/// The NodeSourceMap is a side index of Spans into a NodeTree.
#[derive(Debug, Clone)]
pub struct NodeSourceMap {
    /// The enclosing spans of all nodes. Index is the global node id.
    enclosing_spans: Vec<Span>,
    /// The extra side spans for nodes that have them (sparse).
    side_spans: HashMap<(u32, NodeSpanType), Span>,
}

impl Default for NodeSourceMap {
    fn default() -> Self {
        Self::new()
    }
}

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

    /// Prune spans from the map.
    #[inline]
    pub fn prune_from(&mut self, from_idx: u32) {
        self.enclosing_spans.truncate(from_idx as usize);
        self.side_spans.retain(|&(id, _), _| id < from_idx);
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

    /// Get the span for a node by its id.
    #[inline]
    pub fn get(&self, node_id: u32) -> Span {
        self.enclosing_spans[node_id as usize]
    }

    /// Gets all enclosing spans in the given range (including index).
    #[inline]
    pub fn get_enclosing_spans(&self, start: u32, end_inclusive: u32) -> Vec<EnclosingSpan> {
        let mut spans: Vec<EnclosingSpan> = Vec::new();
        for (i, span) in self.enclosing_spans.iter().enumerate() {
            if span.contains(start) && span.contains(end_inclusive) {
                let distance = (start).abs_diff(span.start) + (span.end).abs_diff(end_inclusive);
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
