use dyst_language_source::Span;

use crate::{Node, NodeId};

/// The NodeMap is a side index into a NodeTree.
#[derive(Debug, Clone)]
pub struct NodeMap {
    /// The spans of all nodes in the AST. Index is the global node id.
    spans_per_node: Vec<Span>,
}

impl Default for NodeMap {
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

impl NodeMap {
    pub fn new() -> Self {
        Self {
            spans_per_node: Vec::new(),
        }
    }

    /// Append a span to the map.
    #[inline]
    pub(crate) fn append_span(&mut self, span: Span) {
        self.spans_per_node.push(span);
    }

    /// Set the span for a node.
    #[inline]
    pub(crate) fn set_span<T: Node>(&mut self, node_id: NodeId<T>, span: Span) {
        self.spans_per_node[node_id.id as usize] = span;
    }

    /// Get the span for a node.
    #[inline]
    pub fn get_span<T: Node>(&self, node_id: NodeId<T>) -> Span {
        self.spans_per_node[node_id.id as usize]
    }

    /// Get the span for a node by its id.
    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Span {
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
