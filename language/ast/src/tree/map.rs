use std::collections::HashMap;

use crate::{CapturingNodeVisitor, Node, LocalNodeId, NodeTree, NodeTreeImpl, walk_any};

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
        for (parent_id, node_type) in tree.type_by_node_id.iter().enumerate() {
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
        for i in 0..tree.type_by_node_id.len() {
            parents_per_node.push(parent_by_node.get(&(i as u32)).cloned());
        }

        Self { parents_per_node }
    }

    /// Get the parent for a node.
    #[inline]
    pub fn get<T>(&self, node_id: LocalNodeId<T>) -> Option<u32>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
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
    pub fn get_ancestors<T>(&self, node_id: LocalNodeId<T>) -> Vec<u32>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.walk_parents_by_id(node_id.id)
    }
}
