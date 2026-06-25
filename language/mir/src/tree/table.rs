use std::marker::PhantomData;

use crate::{LocalNodeId, Node};

/// Dense table keyed by local MIR node ids from one domain.
#[derive(Debug, Clone)]
pub(crate) struct NodeTable<N: Node, T> {
    /// Nodes that belong to this table.
    membership: Vec<bool>,
    /// Values indexed by local node id.
    values: Vec<T>,
    /// The node id domain for this table.
    domain: PhantomData<fn() -> N>,
}

impl<N: Node, T> NodeTable<N, T> {
    /// Create an empty node table.
    pub(crate) fn new() -> Self {
        Self {
            membership: Vec::new(),
            values: Vec::new(),
            domain: PhantomData,
        }
    }

    /// Create a table covering one node set.
    pub(crate) fn from_nodes(nodes: &[LocalNodeId<N>], mut value: impl FnMut() -> T) -> Self {
        let node_count = nodes
            .iter()
            .map(|node| node.id as usize + 1)
            .max()
            .unwrap_or(0);
        let mut membership = vec![false; node_count];
        let mut values = Vec::with_capacity(node_count);
        values.resize_with(node_count, &mut value);

        // mark nodes owned by this table
        for node in nodes {
            membership[node.id as usize] = true;
        }

        Self {
            membership,
            values,
            domain: PhantomData,
        }
    }

    /// Return the number of dense table entries.
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    /// Return whether this table has no dense entries.
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Return all dense values in this table.
    pub(crate) fn values(&self) -> &[T] {
        self.values.as_slice()
    }

    /// Iterate over node ids and values that belong to this table.
    pub(crate) fn iter_nodes(&self) -> impl Iterator<Item = (LocalNodeId<N>, &T)> {
        self.membership
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, is_member)| *is_member)
            .map(|(index, _)| (LocalNodeId::new(index as u32), &self.values[index]))
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

    /// Return the dense index for one node.
    pub(crate) fn index(&self, node: LocalNodeId<N>) -> usize {
        let index = node.id as usize;
        if self.membership.get(index).copied().unwrap_or(false) {
            index
        } else {
            panic!("node is outside table: {node:?}");
        }
    }
}

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
