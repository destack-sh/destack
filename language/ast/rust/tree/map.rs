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

    /// Gets the innermost (smallest-length) enclosing span for a given position.
    pub fn get_innermost_enclosing_span(&self, pos: u32) -> Option<(u32, Span)> {
        let mut best: Option<(u32, Span)> = None;
        for (i, span) in self.spans_per_node.iter().enumerate() {
            if span.contains(pos) {
                match best {
                    None => best = Some((i as u32, *span)),
                    Some((_, current)) => {
                        let cur_len = current.end.saturating_sub(current.start);
                        let new_len = span.end.saturating_sub(span.start);
                        if new_len < cur_len {
                            best = Some((i as u32, *span));
                        }
                    }
                }
            }
        }
        best
    }

    /// Find the first node that starts at or after `pos`.
    /// If multiple nodes start at the same position, prefer the smallest span.
    pub fn get_first_node_starting_after(&self, pos: u32) -> Option<(u32, Span)> {
        let mut candidate: Option<(u32, Span)> = None;
        for (i, span) in self.spans_per_node.iter().enumerate() {
            if span.start >= pos {
                match candidate {
                    None => candidate = Some((i as u32, *span)),
                    Some((_, cur)) => {
                        if span.start < cur.start {
                            candidate = Some((i as u32, *span));
                        } else if span.start == cur.start {
                            let cur_len = cur.end.saturating_sub(cur.start);
                            let new_len = span.end.saturating_sub(span.start);
                            if new_len < cur_len {
                                candidate = Some((i as u32, *span));
                            }
                        }
                    }
                }
            }
        }
        candidate
    }
}
