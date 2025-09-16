use dyst_language_source::Span;

use crate::{Node, NodeId};

/// The NodeMap between Spans and Nodes.
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
    pub idx: u32,
    pub distance: u32,
    pub length: u32,
    pub span: Span,
}

impl NodeMap {
    pub fn new() -> Self {
        Self {
            spans_per_node: Vec::new(),
        }
    }

    #[inline]
    pub fn append_span(&mut self, span: Span) {
        self.spans_per_node.push(span);
    }

    #[inline]
    pub fn set_span<T: Node>(&mut self, node_id: NodeId<T>, span: Span) {
        self.spans_per_node[node_id.id as usize] = span;
    }

    #[inline]
    pub fn get_span<T: Node>(&self, node_id: NodeId<T>) -> Span {
        self.spans_per_node[node_id.id as usize]
    }

    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Span {
        self.spans_per_node[node_id as usize]
    }

    /// Gets all enclosing spans in the given range (including index), sorted by innermost-ness.
    pub fn get_enclosing_spans(&self, start: u32, end: u32) -> Vec<EnclosingSpan> {
        let mut spans: Vec<EnclosingSpan> = Vec::new();
        for (i, span) in self.spans_per_node.iter().enumerate() {
            if span.contains(start) && span.contains(end) {
                let distance = (start).abs_diff(span.start) + (span.end).abs_diff(end);
                spans.push(EnclosingSpan {
                    idx: i as u32,
                    distance,
                    length: span.end.saturating_sub(span.start),
                    span: *span,
                });
            }
        }
        // sort by innermost-ness (smallest length first)
        spans.sort_by_key(|span| span.distance);
        spans
    }
}
