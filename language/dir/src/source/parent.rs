use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{DirectChildCollector, LocalNodeId, LocalNodeIdAny, Node, Tree, TreeStore};

/// Dense structural membership and parent ids keyed by node id.
///
/// Each slot records an unindexed node, a structural root, or a parent node id.
/// Slots use ids relative to the tree base, so a tail tree indexes only its own nodes.
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
    /// Sentinel used for nodes outside the indexed structure.
    const UNINDEXED: u32 = u32::MAX;
    /// Sentinel used for indexed structural roots.
    const ROOT: u32 = u32::MAX - 1;

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

    /// Build a parent index from reachable structural roots.
    pub fn from_roots<T>(tree: &Tree, roots: &[LocalNodeId<T>]) -> Self
    where
        T: Node,
    {
        let mut index = Self::with_base(tree.first_global_id());
        index.index_roots(tree, roots);

        index
    }

    /// Index the structure reachable from further roots, keeping the structure indexed so far.
    pub fn index_roots<T>(&mut self, tree: &Tree, roots: &[LocalNodeId<T>])
    where
        T: Node,
    {
        let index = self;
        let node_count = tree.node_index_by_node_id.len();
        index
            .parent_id_by_node_id
            .resize(node_count, Self::UNINDEXED);
        let mut children = DirectChildCollector::default();
        let mut visited = vec![false; node_count];
        let mut pending = roots.iter().map(|root_id| root_id.id).collect::<Vec<_>>();

        // index the declared structural roots
        for root in roots {
            index.set(root.id, None);
        }

        while let Some(parent_id) = pending.pop() {
            let parent_type = tree.get_node_type(parent_id);
            assert!(
                !tree.is_detached(parent_id),
                "detached DIR {parent_type:?} node {parent_id} is structurally reachable"
            );

            // skip nodes already indexed
            let node_index = tree.node_index(parent_id);
            if visited[node_index] {
                continue;
            }
            visited[node_index] = true;

            // index direct children
            let parent = LocalNodeIdAny::new(parent_id, parent_type);
            let child_ids = index.index_children(tree, parent, &mut children);

            // continue through newly discovered children
            pending.extend_from_slice(child_ids);

            // index side-attached decorators as structural children
            for decorator_id in tree.get_decorators_ref(parent_id) {
                index.index_child(tree, parent, decorator_id.id);
                pending.push(decorator_id.id);
            }
        }
    }

    /// Return the first global node id the index covers.
    pub fn base(&self) -> u32 {
        self.base
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

        (parent_id < Self::ROOT).then_some(parent_id)
    }

    /// Return whether one node belongs to the indexed structure.
    #[inline]
    pub fn contains(&self, node_id: u32) -> bool {
        let Some(index) = node_id.checked_sub(self.base).map(|index| index as usize) else {
            return false;
        };

        self.parent_id_by_node_id
            .get(index)
            .is_some_and(|parent_id| *parent_id != Self::UNINDEXED)
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

    /// Set the parent for one node id, or mark it as a structural root.
    #[inline]
    pub fn set(&mut self, node_id: u32, parent_id: Option<u32>) {
        assert!(
            node_id >= self.base,
            "DIR node id {node_id} is before parent index base {}",
            self.base
        );
        let index = (node_id - self.base) as usize;
        if index >= self.parent_id_by_node_id.len() {
            self.parent_id_by_node_id.resize(index + 1, Self::UNINDEXED);
        }

        self.parent_id_by_node_id[index] = parent_id.unwrap_or(Self::ROOT);
    }

    /// Drop structural entries at or beyond one global node id.
    #[inline]
    pub fn truncate(&mut self, next_global_id: u32) {
        assert!(
            next_global_id >= self.base,
            "DIR node id {next_global_id} is before parent index base {}",
            self.base
        );
        let length = (next_global_id - self.base) as usize;
        assert!(
            self.parent_id_by_node_id
                .iter()
                .take(length)
                .all(|parent_id| *parent_id >= Self::ROOT || *parent_id < next_global_id),
            "retained DIR node has a truncated structural parent"
        );

        self.parent_id_by_node_id.truncate(length);
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
            self.index_child(tree, parent, child_id);
        }

        child_ids
    }

    /// Index one structural child.
    fn index_child(&mut self, tree: &Tree, parent: LocalNodeIdAny, child_id: u32) {
        if self.contains(child_id) {
            let child_type = tree.get_node_type(child_id);
            assert_eq!(
                self.get_by_id(child_id),
                Some(parent.id),
                "DIR {child_type:?} node {child_id} has conflicting structural ownership"
            );
        }

        self.set(child_id, Some(parent.id));
    }
}
