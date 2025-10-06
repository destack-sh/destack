use crate::{Argument, Node, NodeId, NodeType, Path, StringId};

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
        receiver: Path,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// A decorator annotation (like `@foo` or `@foo(1, 2, 3)`).
    Decorator {
        position: AnnotationPosition,
        receiver: Path,
        arguments: Option<Vec<NodeId<Argument>>>,
    },
}

impl Node for Annotation {
    const KIND: NodeType = NodeType::Annotation;
}

impl Annotation {
    /// Get the position of the annotation.
    pub fn position(&self) -> AnnotationPosition {
        match self {
            Annotation::Blank { position, .. } => *position,
            Annotation::Doc { position, .. } => *position,
            Annotation::Comment { position, .. } => *position,
            Annotation::Tag { position, .. } => *position,
            Annotation::Decorator { position, .. } => *position,
        }
    }
}
