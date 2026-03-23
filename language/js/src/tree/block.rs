use crate::{Expression, LocalNodeId, Node, NodeType, Statement};

/// Block of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The statements in the block.
    pub statements: Vec<LocalNodeId<Statement>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

/// A switch case.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    /// The value to match on.
    pub value: LocalNodeId<Expression>,
    /// The body of the case.
    pub body: LocalNodeId<Block>,
}

impl Node for SwitchCase {
    const TYPE: NodeType = NodeType::SwitchCase;
}
