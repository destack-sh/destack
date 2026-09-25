use tspp_core::Arena;

use crate::{Node, NodeId, NodeList};

/// The operations and root of a compiled pattern.
#[derive(Debug)]
pub struct Tree {
    /// The operations in allocation order.
    pub(crate) nodes: Arena<Node>,
    /// The flat children of `All` and `Any` operations.
    pub(crate) node_ids: Vec<NodeId>,
    /// The operation evaluated first.
    pub(crate) root: NodeId,
}

impl Tree {
    /// Return the root operation.
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Return an operation.
    pub fn get(&self, node: NodeId) -> &Node {
        self.nodes.get(node.0)
    }

    /// Return the operations in allocation order.
    pub fn nodes(&self) -> &[Node] {
        self.nodes.as_slice()
    }

    /// Return the children in a list.
    pub fn get_list(&self, nodes: NodeList) -> &[NodeId] {
        let start = nodes.start as usize;
        let end = start + nodes.length as usize;

        &self.node_ids[start..end]
    }
}
