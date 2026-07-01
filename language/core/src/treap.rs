use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{stable_hash_value, stable_hash_value_256};

const EMPTY_TREAP_HASH: [u8; 32] = [0; 32];

/// One persistent treap root over shared immutable nodes.
#[derive(Debug, Clone, Copy)]
pub struct TreapRoot {
    /// The root node id.
    node: Option<TreapNodeId>,
    /// The structural hash for this root.
    hash: [u8; 32],
}

/// Shared immutable persistent map nodes.
#[derive(Debug)]
pub struct Treap<K, V> {
    /// Shared immutable nodes.
    nodes: RwLock<Vec<TreapNode<K, V>>>,
}

/// One node id in a treap arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TreapNodeId(usize);

/// One immutable treap node.
#[derive(Debug, Clone, Copy)]
struct TreapNode<K, V> {
    /// The stable heap priority.
    priority: u64,
    /// The structural hash for this node and its descendants.
    hash: [u8; 32],
    /// The entry key.
    key: K,
    /// The entry value.
    value: V,
    /// The lower key child.
    left: Option<TreapNodeId>,
    /// The higher key child.
    right: Option<TreapNodeId>,
}

impl TreapRoot {
    /// Build an empty root.
    pub const fn new() -> Self {
        Self {
            node: None,
            hash: EMPTY_TREAP_HASH,
        }
    }
}

impl Default for TreapRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for TreapRoot {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for TreapRoot {}

impl Hash for TreapRoot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl<K, V> Treap<K, V> {
    /// Build an empty treap.
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(Vec::new()),
        }
    }

    /// Return one value by key.
    pub fn get(&self, root: TreapRoot, key: &K) -> Option<V>
    where
        K: Ord,
        V: Copy,
    {
        let nodes = self.nodes.read();
        let mut current = root.node;

        // walk the search path
        while let Some(node_id) = current {
            let node = &nodes[node_id.0];
            match key.cmp(&node.key) {
                Ordering::Less => current = node.left,
                Ordering::Equal => return Some(node.value),
                Ordering::Greater => current = node.right,
            }
        }

        None
    }

    /// Return whether the treap contains one key.
    pub fn contains(&self, root: TreapRoot, key: &K) -> bool
    where
        K: Ord,
    {
        let nodes = self.nodes.read();
        let mut current = root.node;

        // walk the search path
        while let Some(node_id) = current {
            let node = &nodes[node_id.0];
            match key.cmp(&node.key) {
                Ordering::Less => current = node.left,
                Ordering::Equal => return true,
                Ordering::Greater => current = node.right,
            }
        }

        false
    }

    /// Return a root with one key inserted or replaced.
    pub fn insert(&self, root: TreapRoot, key: K, value: V) -> TreapRoot
    where
        K: Copy + Hash + Ord,
        V: Copy + Hash,
    {
        let mut nodes = self.nodes.write();
        let node = Self::insert_node(&mut nodes, root.node, key, value);
        let hash = nodes[node.0].hash;

        TreapRoot {
            node: Some(node),
            hash,
        }
    }

    /// Return a root with one key removed.
    pub fn remove(&self, root: TreapRoot, key: &K) -> TreapRoot
    where
        K: Copy + Hash + Ord,
        V: Copy + Hash,
    {
        let mut nodes = self.nodes.write();
        let node = Self::remove_node(&mut nodes, root.node, key);
        let hash = node
            .map(|node| nodes[node.0].hash)
            .unwrap_or(EMPTY_TREAP_HASH);

        TreapRoot { node, hash }
    }

    /// Return copied entries in key order.
    pub fn entries(&self, root: TreapRoot) -> Vec<(K, V)>
    where
        K: Copy,
        V: Copy,
    {
        let nodes = self.nodes.read();
        let mut entries = Vec::new();

        Self::collect_node_entries(&nodes, root.node, &mut entries);

        entries
    }

    /// Return values from nodes reachable through the given roots once.
    pub fn unique_values(&self, roots: impl IntoIterator<Item = TreapRoot>) -> Vec<V>
    where
        V: Copy,
    {
        let nodes = self.nodes.read();
        let mut seen = FxHashSet::default();
        let mut values = Vec::new();

        // collect shared nodes once across all roots
        for root in roots {
            Self::collect_unique_node_values(&nodes, root.node, &mut seen, &mut values);
        }

        values
    }

    /// Retain only nodes reachable from the given roots and rewrite those roots.
    pub fn compact(&self, roots: &mut [TreapRoot])
    where
        K: Copy + Hash,
        V: Copy + Hash,
    {
        let mut nodes = self.nodes.write();
        let mut compacted = Vec::new();
        let mut rewritten = FxHashMap::default();

        // copy reachable nodes into a fresh arena
        for root in roots.iter_mut() {
            root.node = Self::compact_node(&nodes, &mut compacted, &mut rewritten, root.node);
            debug_assert_eq!(
                root.hash,
                root.node
                    .map(|node| compacted[node.0].hash)
                    .unwrap_or(EMPTY_TREAP_HASH)
            );
        }

        *nodes = compacted;
    }

    /// Allocate one shared immutable node.
    fn allocate(
        nodes: &mut Vec<TreapNode<K, V>>,
        priority: u64,
        key: K,
        value: V,
        left: Option<TreapNodeId>,
        right: Option<TreapNodeId>,
    ) -> TreapNodeId
    where
        K: Hash,
        V: Hash,
    {
        let hash = Self::node_hash(nodes, left, &key, &value, right);
        let node = TreapNode {
            priority,
            hash,
            key,
            value,
            left,
            right,
        };
        let id = TreapNodeId(nodes.len());
        nodes.push(node);

        id
    }

    /// Return one node's structural hash.
    fn node_hash(
        nodes: &[TreapNode<K, V>],
        left: Option<TreapNodeId>,
        key: &K,
        value: &V,
        right: Option<TreapNodeId>,
    ) -> [u8; 32]
    where
        K: Hash,
        V: Hash,
    {
        let left = left.map(|id| nodes[id.0].hash).unwrap_or(EMPTY_TREAP_HASH);
        let right = right.map(|id| nodes[id.0].hash).unwrap_or(EMPTY_TREAP_HASH);

        stable_hash_value_256(&(left, key, value, right))
    }

    /// Collect entries in key order.
    fn collect_node_entries(
        nodes: &[TreapNode<K, V>],
        node: Option<TreapNodeId>,
        entries: &mut Vec<(K, V)>,
    ) where
        K: Copy,
        V: Copy,
    {
        let Some(node) = node else {
            return;
        };
        let node = &nodes[node.0];

        // collect in key order
        Self::collect_node_entries(nodes, node.left, entries);
        entries.push((node.key, node.value));
        Self::collect_node_entries(nodes, node.right, entries);
    }

    /// Collect values from treap nodes not yet seen.
    fn collect_unique_node_values(
        nodes: &[TreapNode<K, V>],
        node: Option<TreapNodeId>,
        seen: &mut FxHashSet<TreapNodeId>,
        values: &mut Vec<V>,
    ) where
        V: Copy,
    {
        let Some(node_id) = node else {
            return;
        };
        if !seen.insert(node_id) {
            return;
        }
        let node = &nodes[node_id.0];

        // collect shared descendants once
        Self::collect_unique_node_values(nodes, node.left, seen, values);
        values.push(node.value);
        Self::collect_unique_node_values(nodes, node.right, seen, values);
    }

    /// Insert or replace one key in one node.
    fn insert_node(
        nodes: &mut Vec<TreapNode<K, V>>,
        node: Option<TreapNodeId>,
        key: K,
        value: V,
    ) -> TreapNodeId
    where
        K: Copy + Hash + Ord,
        V: Copy + Hash,
    {
        let Some(node_id) = node else {
            let priority = stable_hash_value(&key);

            return Self::allocate(nodes, priority, key, value, None, None);
        };
        let node = nodes[node_id.0];

        match key.cmp(&node.key) {
            Ordering::Less => {
                let left = Self::insert_node(nodes, node.left, key, value);
                let node = Self::copy_node_with(nodes, node, Some(left), node.right);

                Self::rotate_right_maybe(nodes, node)
            }
            Ordering::Equal => {
                Self::allocate(nodes, node.priority, key, value, node.left, node.right)
            }
            Ordering::Greater => {
                let right = Self::insert_node(nodes, node.right, key, value);
                let node = Self::copy_node_with(nodes, node, node.left, Some(right));

                Self::rotate_left_maybe(nodes, node)
            }
        }
    }

    /// Remove one key from one node.
    fn remove_node(
        nodes: &mut Vec<TreapNode<K, V>>,
        node: Option<TreapNodeId>,
        key: &K,
    ) -> Option<TreapNodeId>
    where
        K: Copy + Hash + Ord,
        V: Copy + Hash,
    {
        let node_id = node?;
        let node = nodes[node_id.0];

        match key.cmp(&node.key) {
            Ordering::Less => {
                let left = Self::remove_node(nodes, node.left, key);

                Some(Self::copy_node_with(nodes, node, left, node.right))
            }
            Ordering::Equal => Self::merge_nodes(nodes, node.left, node.right),
            Ordering::Greater => {
                let right = Self::remove_node(nodes, node.right, key);

                Some(Self::copy_node_with(nodes, node, node.left, right))
            }
        }
    }

    /// Merge two ordered node sets.
    fn merge_nodes(
        nodes: &mut Vec<TreapNode<K, V>>,
        left: Option<TreapNodeId>,
        right: Option<TreapNodeId>,
    ) -> Option<TreapNodeId>
    where
        K: Copy + Hash,
        V: Copy + Hash,
    {
        match (left, right) {
            (None, None) => None,
            (Some(node), None) | (None, Some(node)) => Some(node),
            (Some(left), Some(right)) if nodes[left.0].priority >= nodes[right.0].priority => {
                let left = nodes[left.0];
                let right = Self::merge_nodes(nodes, left.right, Some(right));

                Some(Self::copy_node_with(nodes, left, left.left, right))
            }
            (Some(left), Some(right)) => {
                let right = nodes[right.0];
                let left = Self::merge_nodes(nodes, Some(left), right.left);

                Some(Self::copy_node_with(nodes, right, left, right.right))
            }
        }
    }

    /// Copy one node while replacing its children.
    fn copy_node_with(
        nodes: &mut Vec<TreapNode<K, V>>,
        node: TreapNode<K, V>,
        left: Option<TreapNodeId>,
        right: Option<TreapNodeId>,
    ) -> TreapNodeId
    where
        K: Copy + Hash,
        V: Copy + Hash,
    {
        Self::allocate(nodes, node.priority, node.key, node.value, left, right)
    }

    /// Rotate right when the left child has higher heap priority.
    fn rotate_right_maybe(nodes: &mut Vec<TreapNode<K, V>>, node: TreapNodeId) -> TreapNodeId
    where
        K: Copy + Hash,
        V: Copy + Hash,
    {
        let Some(left_id) = nodes[node.0].left else {
            return node;
        };
        if nodes[left_id.0].priority <= nodes[node.0].priority {
            return node;
        }
        let node = nodes[node.0];
        let left = nodes[left_id.0];
        let right = Self::allocate(
            nodes,
            node.priority,
            node.key,
            node.value,
            left.right,
            node.right,
        );

        Self::allocate(
            nodes,
            left.priority,
            left.key,
            left.value,
            left.left,
            Some(right),
        )
    }

    /// Rotate left when the right child has higher heap priority.
    fn rotate_left_maybe(nodes: &mut Vec<TreapNode<K, V>>, node: TreapNodeId) -> TreapNodeId
    where
        K: Copy + Hash,
        V: Copy + Hash,
    {
        let Some(right_id) = nodes[node.0].right else {
            return node;
        };
        if nodes[right_id.0].priority <= nodes[node.0].priority {
            return node;
        }
        let node = nodes[node.0];
        let right = nodes[right_id.0];
        let left = Self::allocate(
            nodes,
            node.priority,
            node.key,
            node.value,
            node.left,
            right.left,
        );

        Self::allocate(
            nodes,
            right.priority,
            right.key,
            right.value,
            Some(left),
            right.right,
        )
    }

    /// Copy one reachable node into a compacted arena.
    fn compact_node(
        source: &[TreapNode<K, V>],
        target: &mut Vec<TreapNode<K, V>>,
        rewritten: &mut FxHashMap<TreapNodeId, TreapNodeId>,
        node: Option<TreapNodeId>,
    ) -> Option<TreapNodeId>
    where
        K: Copy + Hash,
        V: Copy + Hash,
    {
        let node_id = node?;
        if let Some(rewritten) = rewritten.get(&node_id) {
            return Some(*rewritten);
        }
        let node = source[node_id.0];
        let left = Self::compact_node(source, target, rewritten, node.left);
        let right = Self::compact_node(source, target, rewritten, node.right);
        let rewritten_id = Self::allocate(target, node.priority, node.key, node.value, left, right);

        rewritten.insert(node_id, rewritten_id);

        Some(rewritten_id)
    }
}

impl<K, V> Default for Treap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Treap, TreapRoot};

    /// Preserve map contents and root identity when compacting storage.
    #[test]
    fn test_compact_retain_reachable_root() {
        let treap = Treap::new();
        let mut root = TreapRoot::new();

        // create persistent history
        for key in 0..100_u32 {
            root = treap.insert(root, key, key * 10);
        }
        let root_before = root;
        let nodes_before = treap.nodes.read().len();

        // compact only the final root
        let mut roots = [root];
        treap.compact(&mut roots);
        root = roots[0];
        let nodes_after = treap.nodes.read().len();

        // verify storage shrank to the live tree
        assert_eq!(root, root_before);
        assert_eq!(nodes_after, 100);
        assert!(nodes_after < nodes_before);

        // verify every value remains readable
        for key in 0..100_u32 {
            assert_eq!(treap.get(root, &key), Some(key * 10));
        }
    }

    /// Preserve multiple shared roots when compacting storage.
    #[test]
    fn test_compact_retain_multiple_roots() {
        let treap = Treap::new();
        let mut first = TreapRoot::new();

        // build a base root
        for key in 0..50_u32 {
            first = treap.insert(first, key, key);
        }
        let second = treap.insert(first, 100, 100);
        let first_before = first;
        let second_before = second;

        // compact both roots together
        let mut roots = [first, second];
        treap.compact(&mut roots);
        first = roots[0];
        let second = roots[1];

        // verify root identities and divergent entries
        assert_eq!(first, first_before);
        assert_eq!(second, second_before);
        assert_eq!(treap.get(first, &100), None);
        assert_eq!(treap.get(second, &100), Some(100));
    }
}
