use crate::{Expression, Node, NodeId, NodeType, Statement, StringId};

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub label: Option<StringId>,
    pub statements: Vec<NodeId<Statement>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub value: NodeId<Expression>,
    pub body: NodeId<Block>,
}

impl Node for SwitchCase {
    const TYPE: NodeType = NodeType::SwitchCase;
}
