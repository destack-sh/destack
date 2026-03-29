use crate::{Attribute, Fragment, LocalNodeId, Name, Node, NodeType};
use serde::{Deserialize, Serialize};

/// One HTML node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content {
    /// One element node.
    Element(Element),
    /// One text node.
    Text(Text),
    /// One comment node.
    Comment(Comment),
    /// One instruction node.
    Instruction(Instruction),
}

impl Node for Content {
    const TYPE: NodeType = NodeType::Content;
}

/// One HTML element node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Element {
    /// The element name.
    pub name: Name,
    /// The authored start-tag name spelling when one exists.
    pub authored_start_tag_name: Option<String>,
    /// Whether the element had one authored end tag.
    pub has_authored_end_tag: bool,
    /// The authored end-tag name spelling when one exists.
    pub authored_end_tag_name: Option<String>,
    /// Whether the authored start tag used self-closing syntax.
    pub is_self_closing: bool,
    /// The authored self-closing slash form when one exists.
    pub self_closing_style: Option<SelfClosingStyle>,
    /// The attributes in source order.
    pub attributes: Vec<LocalNodeId<Attribute>>,
    /// The child nodes.
    pub children: Vec<LocalNodeId<Content>>,
    /// The template contents fragment when one exists.
    pub content: Option<LocalNodeId<Fragment>>,
}

/// One authored self-closing slash form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfClosingStyle {
    /// One compact `/>` close.
    Compact,
    /// One spaced ` />` close.
    Spaced,
}

/// One HTML text node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text {
    /// The authored text contents.
    pub value: String,
}

/// One HTML comment node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    /// The authored comment contents.
    pub value: String,
}

/// One HTML instruction node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instruction {
    /// The instruction target.
    pub target: String,
    /// The instruction contents.
    pub contents: String,
}
