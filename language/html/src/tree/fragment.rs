use crate::{Content, LocalNodeId, Node, NodeType};
use serde::{Deserialize, Serialize};

/// One HTML document fragment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fragment {
    /// The fragment children.
    pub children: Vec<LocalNodeId<Content>>,
}

impl Node for Fragment {
    const TYPE: NodeType = NodeType::Fragment;
}
