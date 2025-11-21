use crate::{Argument, GlobalSymbolId, LocalNodeId, Node, NodeType, Path, StringId};

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
    /// Doc annotation (like `///` or `/**`).
    Doc {
        position: AnnotationPosition,
        string: StringId,
    },
    /// Comment annotation (like `//` or `/*`).
    Comment {
        position: AnnotationPosition,
        string: StringId,
    },
    /// Unresolved tag annotation (like `#Foo` or `#Foo(x: 1)`).
    UnresolvedTag {
        position: AnnotationPosition,
        left: Path,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Unresolved decorator annotation (like `@foo` or `@foo(1, 2, 3)`).
    UnresolvedDecorator {
        position: AnnotationPosition,
        left: Path,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Tag annotation (like `#Foo` or `#Foo(x: 1)`).
    Tag {
        position: AnnotationPosition,
        left: Path,
        symbol: GlobalSymbolId,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Decorator annotation (like `@foo` or `@foo(1, 2, 3)`).
    Decorator {
        position: AnnotationPosition,
        left: Path,
        symbol: GlobalSymbolId,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;

    fn is_resolved(&self) -> bool {
        matches!(
            self,
            Annotation::Doc { .. }
                | Annotation::Comment { .. }
                | Annotation::Tag { .. }
                | Annotation::Decorator { .. }
        )
    }
}

impl Annotation {
    /// Get the position of the annotation.
    pub fn position(&self) -> AnnotationPosition {
        match self {
            Annotation::Doc { position, .. } => *position,
            Annotation::Comment { position, .. } => *position,
            Annotation::UnresolvedTag { position, .. } => *position,
            Annotation::UnresolvedDecorator { position, .. } => *position,
            Annotation::Tag { position, .. } => *position,
            Annotation::Decorator { position, .. } => *position,
        }
    }
}
