use crate::{Node, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
}

impl Node for Statement {
    const TYPE: NodeType = NodeType::Statement;
}