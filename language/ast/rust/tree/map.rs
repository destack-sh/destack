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

    /// Gets the smallest enclosing span for a given position.
    pub fn get_enclosing_span(&self, pos: u32) -> Option<(u32, Span)> {
        let mut min_distance = u32::MAX;
        let mut enclosing_span: Option<(u32, Span)> = None;
        for (i, span) in self.spans_per_node.iter().enumerate() {
            if span.contains(pos) {
                let distance = pos.saturating_sub(span.start);
                if distance < min_distance {
                    min_distance = distance;
                    enclosing_span = Some((i as u32, *span));
                }
            }
        }
        enclosing_span
    }
}
