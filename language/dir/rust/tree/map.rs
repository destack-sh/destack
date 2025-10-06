use std::collections::HashMap;

use dyst_source::Span;

use crate::{CapturingNodeVisitor, Node, NodeId, NodeTree, NodeTreeStore, walk_any};

/// The type of node search to perform.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum NodeSearch {
    /// Search for the biggest outermost node that matches.
    Outer,
    /// Search for the smallest outermost node that matches.
    Inner,
}

/// The NodeSpanIndex is a side index of Spans into a NodeTree.
#[derive(Debug, Clone)]
pub struct NodeSpanIndex {
    /// The spans of all nodes. Index is the global node id.
    spans_per_node: Vec<Span>,
}

impl Default for NodeSpanIndex {
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

impl NodeSpanIndex {
    pub fn new() -> Self {
        Self {
            spans_per_node: Vec::new(),
        }
    }

    /// Append a span to the map.
    #[inline]
    pub(crate) fn append(&mut self, span: Span) {
        self.spans_per_node.push(span);
    }

    /// Prune spans from the map.
    #[inline]
    pub(crate) fn prune_from(&mut self, from_idx: u32) {
        self.spans_per_node.truncate(from_idx as usize);
    }

    /// Set the span for a node.
    #[inline]
    pub(crate) fn set<T: Node>(&mut self, node_id: NodeId<T>, span: Span) {
        self.spans_per_node[node_id.id as usize] = span;
    }

    /// Get the span for a node.
    #[inline]
    pub fn get<T: Node>(&self, node_id: NodeId<T>) -> Span {
        self.spans_per_node[node_id.id as usize]
    }

    /// Get the span for a node by its id.
    #[inline]
    pub fn get_by_id(&self, node_id: u32) -> Span {
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

/// The NodeParentIndex is a side index of parent nodes into a NodeTree.
#[derive(Debug, Clone)]
pub struct NodeParentIndex {
    parents_per_node: Vec<Option<u32>>,
}

impl NodeParentIndex {
    /// Create a new NodeParentIndex from a NodeTree.
    pub fn from_tree(tree: &NodeTree) -> Self {
        let mut capturing_visitor = CapturingNodeVisitor::default();
        let mut parent_by_node: HashMap<u32, u32> = HashMap::new();

        // capture the parents of each node
        for (parent_id, node_type) in tree.type_by_node.iter().enumerate() {
            capturing_visitor.reset();
            walk_any(&mut capturing_visitor, tree, *node_type, parent_id as u32);
            for node_id in capturing_visitor.visited() {
                if *node_id != parent_id as u32 {
                    // ignore self
                    parent_by_node.insert(*node_id, parent_id as u32);
                }
            }
        }

        // put into linear map
        let mut parents_per_node: Vec<Option<u32>> = Vec::new();
        for i in 0..tree.type_by_node.len() {
            parents_per_node.push(parent_by_node.get(&(i as u32)).cloned());
        }

        Self { parents_per_node }
    }

    /// Get the parent for a node.
    #[inline]
    pub fn get<T>(&self, node_id: NodeId<T>) -> Option<u32>
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        self.parents_per_node[node_id.id as usize]
    }

    /// Get the parent for a node by its id.
    #[inline]
    pub fn get_by_id(&self, node_id: u32) -> Option<u32> {
        self.parents_per_node[node_id as usize]
    }

    /// Walk all parents to the root.
    #[inline]
    pub fn walk_parents_by_id(&self, node_id: u32) -> Vec<u32> {
        let mut parents: Vec<u32> = Vec::new();
        let mut current_id = node_id;
        while let Some(parent_id) = self.get_by_id(current_id) {
            parents.push(parent_id);
            current_id = parent_id;
        }
        parents
    }

    /// Walk all parents to the root.
    #[inline]
    pub fn get_ancestors<T>(&self, node_id: NodeId<T>) -> Vec<u32>
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        self.walk_parents_by_id(node_id.id)
    }
}
