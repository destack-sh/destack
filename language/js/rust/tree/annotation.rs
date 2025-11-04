use crate::{Node, NodeType, StringId};

#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    /// A comment annotation.
    Comment { string: StringId },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;
}
