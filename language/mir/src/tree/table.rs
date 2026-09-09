use std::marker::PhantomData;

use crate::{LocalNodeId, Node};

/// Compact table keyed by local MIR node ids from one domain.
#[derive(Debug, Clone)]
pub(crate) struct NodeTable<N: Node, T> {
    /// Sorted node ids parallel to the values.
    nodes: Vec<LocalNodeId<N>>,
    /// Values indexed by node position.
    values: Vec<T>,
    /// The node id domain for this table.
    domain: PhantomData<fn() -> N>,
}

impl<N: Node, T> NodeTable<N, T> {
    /// Create an empty node table.
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            values: Vec::new(),
            domain: PhantomData,
        }
    }

    /// Create a table covering one node set.
    pub(crate) fn from_nodes(nodes: &[LocalNodeId<N>], mut value: impl FnMut() -> T) -> Self {
        let mut nodes = nodes.to_vec();
        nodes.sort_unstable_by_key(|node| node.id);

        // reject duplicate entries in one node domain
        if nodes.windows(2).any(|nodes| nodes[0] == nodes[1]) {
            unreachable!("duplicate node in table");
        }

        let mut values = Vec::with_capacity(nodes.len());
        values.resize_with(nodes.len(), &mut value);

        Self {
            nodes,
            values,
            domain: PhantomData,
        }
    }

    /// Return all dense values in this table.
    pub(crate) fn values(&self) -> &[T] {
        self.values.as_slice()
    }

    /// Return one table entry.
    pub(crate) fn get(&self, node: LocalNodeId<N>) -> &T {
        let index = self.index(node);

        &self.values[index]
    }

    /// Return one mutable table entry.
    pub(crate) fn get_mut(&mut self, node: LocalNodeId<N>) -> &mut T {
        let index = self.index(node);

        &mut self.values[index]
    }

    /// Return one node's compact table index.
    fn index(&self, node: LocalNodeId<N>) -> usize {
        self.nodes
            .binary_search_by_key(&node.id, |candidate| candidate.id)
            .unwrap_or_else(|_| unreachable!("node is outside table: {node:?}"))
    }
}

impl<N: Node, T: PartialEq> PartialEq for NodeTable<N, T> {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes && self.values == other.values
    }
}

impl<N: Node, T: Eq> Eq for NodeTable<N, T> {}

impl<N: Node, T: Copy> NodeTable<N, Option<T>> {
    /// Return one expected optional table entry.
    pub(crate) fn expect(&self, node: LocalNodeId<N>) -> T {
        match *self.get(node) {
            Some(value) => value,
            None => unreachable!("missing expected node table entry for {node:?}"),
        }
    }
}

impl<N: Node, T> Default for NodeTable<N, T> {
    fn default() -> Self {
        Self::new()
    }
}
