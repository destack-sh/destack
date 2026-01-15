use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, LocalScopeId, Node, NodeType};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// The scope of the block.
    pub scope: LocalScopeId,
    /// The expressions in the block.
    pub expressions: Vec<LocalNodeId<Expression>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}
