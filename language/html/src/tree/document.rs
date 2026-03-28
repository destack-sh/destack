use crate::{Content, LocalNodeId, Node, NodeType};
use serde::{Deserialize, Serialize};

/// One HTML document node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// The document type when one exists.
    pub doctype: Option<LocalNodeId<Doctype>>,
    /// The top-level document children.
    pub children: Vec<LocalNodeId<Content>>,
}

impl Node for Document {
    const TYPE: NodeType = NodeType::Document;
}

/// One HTML doctype node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Doctype {
    /// The authored doctype name.
    pub name: String,
    /// The authored public id.
    pub public_id: String,
    /// The authored system id.
    pub system_id: String,
}

impl Node for Doctype {
    const TYPE: NodeType = NodeType::Doctype;
}
