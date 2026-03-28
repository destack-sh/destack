use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use crate::{Fragment, Name};
use serde::{Deserialize, Serialize};

/// The type of one HTML node.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeType {
    Document,
    Doctype,
    Fragment,
    Content,
    Attribute,
}

impl NodeType {
    /// Return the human readable node type name.
    pub fn name(&self) -> &'static str {
        match self {
            NodeType::Document => "document",
            NodeType::Doctype => "doctype",
            NodeType::Fragment => "fragment",
            NodeType::Content => "html node",
            NodeType::Attribute => "attribute",
        }
    }
}

/// One untyped local node id.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalNodeIdAny {
    /// The raw node id.
    pub id: u32,
    /// The node type.
    pub ty: NodeType,
}

impl LocalNodeIdAny {
    /// Create one untyped local node id.
    pub fn new(id: u32, ty: NodeType) -> Self {
        Self { id, ty }
    }
}

impl Debug for LocalNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeIdAny")
            .field("id", &self.id)
            .field("type", &self.ty)
            .finish()
    }
}

/// One typed local node id.
#[repr(transparent)]
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalNodeId<T: Node> {
    /// The raw node id.
    pub id: u32,
    #[serde(skip)]
    _ty: PhantomData<fn() -> T>,
}

impl<T: Node> LocalNodeId<T> {
    /// Create one typed local node id.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            _ty: PhantomData,
        }
    }

    /// Convert this node id into its untyped form.
    pub fn into_any(self) -> LocalNodeIdAny {
        LocalNodeIdAny {
            id: self.id,
            ty: T::TYPE,
        }
    }
}

impl<T: Node> Debug for LocalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeId").field("id", &self.id).finish()
    }
}

impl<T: Clone + Node> Copy for LocalNodeId<T> {}

impl<T: Node> From<LocalNodeId<T>> for LocalNodeIdAny {
    fn from(id: LocalNodeId<T>) -> Self {
        id.into_any()
    }
}

/// One typed node.
pub trait Node: Sized {
    const TYPE: NodeType;
}

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
    /// The attributes in source order.
    pub attributes: Vec<LocalNodeId<crate::Attribute>>,
    /// The child nodes.
    pub children: Vec<LocalNodeId<Content>>,
    /// The template contents fragment when one exists.
    pub content: Option<LocalNodeId<Fragment>>,
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
