use crate::{Expression, LocalNodeId, LocalScopeId, Node, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The scope of the block.
    pub scope: LocalScopeId,
    /// The expressions in the block.
    pub expressions: Vec<LocalNodeId<Expression>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}
