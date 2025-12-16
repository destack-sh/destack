use std::collections::HashMap;

use crate::Span;

/// The type of node search to perform.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum NodeSearch {
    /// Search for the biggest outermost node that matches.
    BiggestOutermost,
    /// Search for the smallest outermost node that matches.
    SmallestOutermost,
    /// Search for the smallest innermost node that matches.
    SmallestInnermost,
}

/// The NodeSourceMap is a side index of Spans into a NodeTree.
#[derive(Debug, Clone)]
pub struct NodeSourceMap {
    /// The spans of all nodes. Index is the global node id.
    spans_per_node: Vec<Span>,
    /// The "main" spans for nodes that have them (sparse).
    /// For declarations, this is the identifier span.
    /// For operators, this is the operator token span.
    main_spans: HashMap<u32, Span>,
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
            spans_per_node: Vec::new(),
            main_spans: HashMap::new(),
        }
    }

    /// Append a span to the map.
    #[inline]
    pub fn append(&mut self, span: Span) {
        self.spans_per_node.push(span);
    }

    /// Set the span for a node.
    #[inline]
    pub fn set(&mut self, node_id: u32, span: Span) {
        self.spans_per_node[node_id as usize] = span;
    }

    /// Prune spans from the map.
    #[inline]
    pub fn prune_from(&mut self, from_idx: u32) {
        self.spans_per_node.truncate(from_idx as usize);
        self.main_spans.retain(|&id, _| id < from_idx);
    }

    /// Set the main span for a node.
    #[inline]
    pub fn set_main(&mut self, node_id: u32, span: Span) {
        self.main_spans.insert(node_id, span);
    }

    /// Get the main span for a node, if it has one.
    #[inline]
    pub fn get_main(&self, node_id: u32) -> Option<Span> {
        self.main_spans.get(&node_id).copied()
    }

    /// Get the span for a node by its id.
    #[inline]
    pub fn get(&self, node_id: u32) -> Span {
        self.spans_per_node[node_id as usize]
    }

    /// Gets all enclosing spans in the given range (including index).
    #[inline]
    pub fn get_enclosing_spans(&self, start: u32, end_inclusive: u32) -> Vec<EnclosingSpan> {
        let mut spans: Vec<EnclosingSpan> = Vec::new();
        for (i, span) in self.spans_per_node.iter().enumerate() {
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
