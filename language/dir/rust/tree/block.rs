use crate::{Expression, Node, NodeId, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Block {
    const KIND: NodeType = NodeType::Block;
}
