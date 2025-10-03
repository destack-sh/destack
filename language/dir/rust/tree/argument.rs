use dyst_ast::StringId;

use crate::{Expression, Node, NodeId, NodeType};

#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub name: StringId,
    pub value: NodeId<Expression>,
}

impl Node for Argument {
    const KIND: NodeType = NodeType::Argument;
}
