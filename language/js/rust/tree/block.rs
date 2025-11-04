use crate::{Node, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}