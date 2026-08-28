use crate::{Expression, LocalNodeId, Node, NodeType, Pattern, Statement};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Block of statements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Block {
    /// The statements in the block.
    pub statements: Vec<LocalNodeId<Statement>>,
}

impl Node for Block {
    const TYPE: NodeType = NodeType::Block;
}

/// One catch clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CatchClause {
    /// The optional catch pattern.
    pub pattern: Option<LocalNodeId<Pattern>>,
    /// The catch body.
    pub body: LocalNodeId<Block>,
}

impl Node for CatchClause {
    const TYPE: NodeType = NodeType::CatchClause;
}

/// One switch case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SwitchCase {
    /// The optional case selector.
    pub value: Option<LocalNodeId<Expression>>,
    /// The statements selected by the case.
    pub body: Vec<LocalNodeId<Statement>>,
}

impl Node for SwitchCase {
    const TYPE: NodeType = NodeType::SwitchCase;
}
