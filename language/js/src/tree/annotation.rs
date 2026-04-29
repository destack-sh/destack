use crate::{Node, NodeType, StringId};

use serde::{Deserialize, Serialize};
/// The position of a JS annotation.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnnotationPosition {
    /// Before the node.
    Prefix,
    /// Inside the node.
    Infix,
    /// After the node.
    Postfix,
}

/// Annotation to a JS node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Annotation {
    /// Comment annotation (like `//` or `/*`).
    Comment {
        position: AnnotationPosition,
        string: StringId,
    },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;
}
