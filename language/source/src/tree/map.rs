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

/// The FileSourceMap is a side index of Spans into a NodeTree.
#[derive(Debug, Clone)]
pub struct FileSourceMap {
    /// The spans of all nodes. Index is the global node id.
    spans_per_node: Vec<Span>,
}

impl Default for FileSourceMap {
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

impl FileSourceMap {
    pub fn new() -> Self {
        Self {
            spans_per_node: Vec::new(),
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
