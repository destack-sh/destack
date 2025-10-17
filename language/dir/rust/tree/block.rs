use crate::{Expression, Node, NodeId, NodeType, StringId};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The label of the block.
    pub label: Option<StringId>,
    /// The expressions in the block.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Block {
    const KIND: NodeType = NodeType::Block;
}