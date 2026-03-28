use crate::{Name, Node, NodeType};
use serde::{Deserialize, Serialize};

/// One HTML attribute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    /// The attribute name.
    pub name: Name,
    /// The attribute value when one exists.
    pub value: Option<String>,
}

impl Node for Attribute {
    const TYPE: NodeType = NodeType::Attribute;
}
