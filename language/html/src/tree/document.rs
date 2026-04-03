use crate::{Content, LocalNodeId, Node, NodeType, StringId};
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
    pub name: StringId,
    /// The authored `doctype` keyword spelling.
    pub doctype_keyword: StringId,
    /// The authored doctype keyword form.
    pub kind: DoctypeKind,
    /// The authored `public` or `system` keyword spelling when one exists.
    pub kind_keyword: Option<StringId>,
    /// The authored public id.
    pub public_id: String,
    /// The authored public id quote style when one exists.
    pub public_id_quote_style: Option<DoctypeQuoteStyle>,
    /// The authored system id.
    pub system_id: String,
    /// The authored system id quote style when one exists.
    pub system_id_quote_style: Option<DoctypeQuoteStyle>,
}

impl Node for Doctype {
    const TYPE: NodeType = NodeType::Doctype;
}

/// One authored doctype keyword form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoctypeKind {
    /// One bare `<!doctype name>` form.
    NameOnly,
    /// One `PUBLIC` doctype form.
    Public,
    /// One `SYSTEM` doctype form.
    System,
}

/// One authored doctype quote style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoctypeQuoteStyle {
    /// One double-quoted id.
    DoubleQuoted,
    /// One single-quoted id.
    SingleQuoted,
}
