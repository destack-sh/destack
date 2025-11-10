use crate::{Expression, Node, NodeId, NodeType, Statement, StringId};

/// Block of statements.
/// NOTE: JS technically supports labels on any statement, but we only support them on blocks (for now).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The label of the block.
    pub label: Option<StringId>,
    /// The statements in the block.
    pub statements: Vec<NodeId<Statement>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

/// A switch case.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    /// The value to match on.
    pub value: NodeId<Expression>,
    /// The body of the case.
    pub body: NodeId<Block>,
}

impl Node for SwitchCase {
    const TYPE: NodeType = NodeType::SwitchCase;
}
