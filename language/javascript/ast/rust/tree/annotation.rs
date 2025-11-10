use crate::{Node, NodeType, StringId};

/// Annotation to a JS node (like a comment or doc comment).
#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    /// Documentation annotation (like `/**`).
    Doc { string: StringId },
    /// Comment annotation (like `//` or `/*`).
    Comment { string: StringId },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;
}
