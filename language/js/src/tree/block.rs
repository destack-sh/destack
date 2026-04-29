use crate::{Expression, LocalNodeId, Node, NodeType, Statement};

use serde::{Deserialize, Serialize};
/// Block of statements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// The statements in the block.
    pub statements: Vec<LocalNodeId<Statement>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

/// A catch clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatchClause {
    /// The optional catch pattern.
    pub pattern: Option<LocalNodeId<crate::Pattern>>,
    /// The catch body.
    pub body: LocalNodeId<Block>,
}

impl Node for CatchClause {
    const TYPE: NodeType = NodeType::CatchClause;
}

/// A switch case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitchCase {
    /// The optional case selector.
    pub value: Option<LocalNodeId<Expression>>,
    /// The body of the case.
    pub body: LocalNodeId<Block>,
}

impl Node for SwitchCase {
    const TYPE: NodeType = NodeType::SwitchCase;
}
