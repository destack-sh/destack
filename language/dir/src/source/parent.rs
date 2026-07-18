use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{DirectChildCollector, Expression, LocalNodeId, LocalNodeIdAny, Node, Tree, TreeStore};

/// Dense structural parent ids keyed by node id.
///
/// Slots use node ids relative to the tree base, matching the tree's other dense storage.
/// A tail tree therefore indexes only its own nodes.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct NodeParentIndex {
    /// The first global node id covered by the index.
    base: u32,
    /// The parent id for each relative node id.
    parent_id_by_node_id: Vec<u32>,
}

impl Default for NodeParentIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeParentIndex {
    /// Sentinel used for nodes without a parent.
    const NO_PARENT: u32 = u32::MAX;

    /// Create a new NodeParentIndex over a base tree.
    pub fn new() -> Self {
        Self::with_base(0)
    }

    /// Create a new NodeParentIndex over a tree starting at one base node id.
    pub fn with_base(base: u32) -> Self {
        Self {
            base,
            parent_id_by_node_id: Vec::new(),
        }
    }

    /// Create a new NodeParentIndex from a Tree.
    pub fn from_tree(tree: &Tree) -> Self {
        // build the dense parent lookup from every node's direct children
        let base = tree.first_global_id();
        let node_count = tree.node_index_by_node_id.len();
        let mut index = Self::with_base(base);
        index
            .parent_id_by_node_id
            .resize(node_count, Self::NO_PARENT);
        let mut children = DirectChildCollector::default();
        for (node_index, entry) in tree.node_index_by_node_id.iter().enumerate() {
            let node_id = base + node_index as u32;
            let node = LocalNodeIdAny::new(node_id, entry.node_type());
            index.index_children(tree, node, &mut children);
        }

        // record side-attached decorators against the nodes they decorate
        for (owner_id, decorator_ids) in tree.get_all_decorators() {
            for decorator_id in decorator_ids {
                if let Some(existing_parent) = index.get(*decorator_id) {
                    assert_eq!(
                        existing_parent, *owner_id,
                        "DIR decorator node {} has multiple structural parents",
                        decorator_id.id
                    );
                }
                index.set(decorator_id.id, Some(*owner_id));
            }
        }

        index
    }

    /// Create a new NodeParentIndex from reachable expression roots.
    pub fn from_expression_roots(tree: &Tree, roots: &[LocalNodeId<Expression>]) -> Self {
        let base = tree.first_global_id();
        let node_count = tree.node_index_by_node_id.len();
        let mut index = Self::with_base(base);
        index
            .parent_id_by_node_id
            .resize(node_count, Self::NO_PARENT);
        let mut children = DirectChildCollector::default();
        let mut visited = vec![false; node_count];
        let mut pending = roots.iter().map(|root_id| root_id.id).collect::<Vec<_>>();

        while let Some(parent_id) = pending.pop() {
            // skip nodes already indexed
            let node_index = tree.node_index(parent_id);
            if visited[node_index] {
                continue;
            }
            visited[node_index] = true;

            // index direct children
            let node_type = tree.get_node_type(parent_id);
            let parent = LocalNodeIdAny::new(parent_id, node_type);
            let child_ids = index.index_children(tree, parent, &mut children);

            // continue through newly discovered children
            pending.extend_from_slice(child_ids);
        }

        index
    }

    /// Get the parent for a node.
    #[inline]
    pub fn get<T>(&self, node_id: LocalNodeId<T>) -> Option<u32>
    where
        T: Node,
    {
        self.get_by_id(node_id.id)
    }

    /// Get the parent for a node by its id.
    #[inline]
    pub fn get_by_id(&self, node_id: u32) -> Option<u32> {
        let index = node_id.checked_sub(self.base)? as usize;
        let parent_id = *self.parent_id_by_node_id.get(index)?;
        (parent_id != Self::NO_PARENT).then_some(parent_id)
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
    pub fn get_ancestors<T>(&self, node_id: LocalNodeId<T>) -> Vec<u32>
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.walk_parents_by_id(node_id.id)
    }

    /// Set or clear the parent for one node id, growing the index to fit.
    #[inline]
    pub fn set(&mut self, node_id: u32, parent_id: Option<u32>) {
        assert!(
            node_id >= self.base,
            "DIR node id {node_id} is before parent index base {}",
            self.base
        );
        let index = (node_id - self.base) as usize;
        if index >= self.parent_id_by_node_id.len() {
            self.parent_id_by_node_id.resize(index + 1, Self::NO_PARENT);
        }

        self.parent_id_by_node_id[index] = parent_id.unwrap_or(Self::NO_PARENT);
    }

    /// Drop parents for nodes at or beyond one global id and clear dangling links.
    #[inline]
    pub fn truncate(&mut self, next_global_id: u32) {
        assert!(
            next_global_id >= self.base,
            "DIR node id {next_global_id} is before parent index base {}",
            self.base
        );
        let length = (next_global_id - self.base) as usize;
        self.parent_id_by_node_id.truncate(length);

        // clear links into the truncated node range
        for parent_id in &mut self.parent_id_by_node_id {
            if *parent_id != Self::NO_PARENT && *parent_id >= next_global_id {
                *parent_id = Self::NO_PARENT;
            }
        }
    }

    /// Index the direct children of one structural parent.
    fn index_children<'a>(
        &mut self,
        tree: &Tree,
        parent: LocalNodeIdAny,
        children: &'a mut DirectChildCollector,
    ) -> &'a [u32] {
        let child_ids = children.collect(tree, parent);
        for child_id in child_ids.iter().copied() {
            if let Some(existing_parent) = self.get_by_id(child_id) {
                assert_eq!(
                    existing_parent, parent.id,
                    "DIR node {child_id} has multiple structural parents"
                );
            }
            self.set(child_id, Some(parent.id));
        }

        child_ids
    }
}
