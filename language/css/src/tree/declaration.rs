use crate::{DeclarationValue, LocalNodeId, Node, NodeType};
use serde::{Deserialize, Serialize};

/// One CSS declaration block node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarationBlock {
    /// The declarations in authored order.
    pub declarations: Vec<LocalNodeId<Declaration>>,
}

impl Node for DeclarationBlock {
    const TYPE: NodeType = NodeType::DeclarationBlock;
}

/// One CSS declaration node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declaration {
    /// The property name.
    pub name: PropertyName,
    /// The property value without one trailing semicolon.
    pub value: DeclarationValue,
    /// Whether this declaration is marked important.
    pub is_important: bool,
}

impl Node for Declaration {
    const TYPE: NodeType = NodeType::Declaration;
}

/// One authored CSS property name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyName {
    /// One known property name.
    Standard(String),
    /// One custom property name.
    Custom(String),
}
