use crate::{Argument, Expression, LocalNodeId, Node, NodeType, StringId};

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

/// An annotation attached to a DIR node.
///
/// Annotations include documentation, comments, and decorators (metadata/transformations).
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

    /// Decorator annotation (like `@foo`, `@foo()`, or `@obj.method(args)`).
    ///
    /// Decorators serve both as metadata (when resolving to a newtype) and
    /// transformations (when resolving to a function).
    ///
    /// `left` can be any expression (Path, MemberAccess, etc.) and `arguments`
    /// are the decorator's own arguments (for factory patterns or metadata values).
    ///
    /// Semantics at runtime (when decorator resolves to a function):
    /// - `@foo` (no args) → `foo(target)`
    /// - `@foo(x)` (with args) → `foo(x)(target)` (factory pattern)
    /// - `@obj.method` → `obj.method(target)`
    ///
    /// When decorator resolves to a newtype, it becomes compile-time metadata
    /// that is stripped in the TS/JS output but available for reflection.
    ///
    /// Examples:
    /// - `@memoize` - simple decorator (transformation)
    /// - `@route("/api")` - decorator factory (transformation)
    /// - `@deprecated("use new API")` - metadata decorator
    /// - `@internal` - metadata decorator (stripped from output)
    Decorator {
        position: AnnotationPosition,
        left: LocalNodeId<Expression>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
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
            Annotation::Decorator { position, .. } => *position,
        }
    }
}
