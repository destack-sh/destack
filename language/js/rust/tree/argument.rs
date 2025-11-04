use crate::{Node, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub enum Argument {}

impl Node for Argument {
    const TYPE: NodeType = NodeType::Argument;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {}

impl Node for Parameter {
    const TYPE: NodeType = NodeType::Parameter;
}
