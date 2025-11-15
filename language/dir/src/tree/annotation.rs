use crate::{Argument, Node, NodeId, NodeType, Path, StringId};

/// The position of an annotation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AnnotationPosition {
    /// Before the node.
    Prefix,
    /// Inside the node.
    Infix,
    /// After the node.
    Postfix,
}

/// An annotation to a DIR node (like a comment or doc comment).
#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
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
    const TYPE: NodeType = NodeType::Annotation;
}

impl Annotation {
    /// Get the position of the annotation.
    pub fn position(&self) -> AnnotationPosition {
        match self {
            Annotation::Doc { position, .. } => *position,
            Annotation::Comment { position, .. } => *position,
            Annotation::Tag { position, .. } => *position,
            Annotation::Decorator { position, .. } => *position,
        }
    }

    /// Get the receiver path of the annotation.
    pub fn receiver(&self) -> Option<&Path> {
        match self {
            Annotation::Tag { receiver, .. } => Some(receiver),
            Annotation::Decorator { receiver, .. } => Some(receiver),
            _ => None,
        }
    }

    /// Whether this annotation is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        match self.receiver() {
            Some(receiver) => receiver.is_resolved(),
            None => true,
        }
    }
}
