use crate::{Node, NodeType, StringId};

#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    /// Documentation annotation.
    Doc { string: StringId },
    /// Comment annotation.
    Comment { string: StringId },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;
}
