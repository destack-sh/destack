use crate::{Node, NodeType, StringId};

/// The position of a JS annotation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AnnotationPosition {
    /// Before the node.
    Prefix,
    /// Inside the node.
    Infix,
    /// After the node.
    Postfix,
}

/// Annotation to a JS node (like a comment or doc comment).
#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    /// Documentation annotation (like `/**`).
    Doc {
        position: AnnotationPosition,
        string: StringId,
    },
    /// Comment annotation (like `//` or `/*`).
    Comment {
        position: AnnotationPosition,
        string: StringId,
    },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;
}
