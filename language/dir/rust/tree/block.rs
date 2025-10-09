use crate::{Expression, Node, NodeId, NodeType, StringId};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub label: Option<StringId>,
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Block {
    const KIND: NodeType = NodeType::Block;
}
