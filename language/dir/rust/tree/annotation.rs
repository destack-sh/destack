use crate::path::PathId;
use crate::{Argument, Node, NodeId, NodeType, StringId};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AnnotationPosition {
    /// Before the node.
    Prefix,
    /// Inside the node.
    Infix,
    /// After the node.
    Postfix,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    /// A blank annotation (just newlines).
    Blank {
        position: AnnotationPosition,
        lines: u32,
    },
    /// A doc annotation (like `///` or `/**`).
    Doc {
        position: AnnotationPosition,
        string: StringId,
    },
    /// A comment annotation (like `//` or `/*`).
    Comment {
        position: AnnotationPosition,
        string: StringId,
    },
    /// A tag annotation (like `#Foo` or `#Foo(x: 1)`).
    Tag {
        position: AnnotationPosition,
        receiver: PathId,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
    Decorator {
        position: AnnotationPosition,
        receiver: PathId,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
}

impl Node for Annotation {
    const KIND: NodeType = NodeType::Annotation;
}
