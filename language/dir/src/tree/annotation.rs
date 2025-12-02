use crate::{Argument, Expression, LocalNodeId, Node, NodeType, Path, StaticExpression, StringId};

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
/// Annotations include documentation, comments, tags (metadata), and decorators (transformations).
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

    /// Unevaluated tag annotation (like `#Foo` or `#Foo(1, 2)`).
    ///
    /// Tags are metadata attached to declarations. The tag path must resolve to a newtype
    /// or a function that returns a static value. After type analysis and elaboration,
    /// this becomes a `Tag` with the evaluated value.
    ///
    /// Examples:
    /// - `#Performance` - unit newtype tag
    /// - `#deprecated("use new API")` - scalar newtype tag
    /// - `#version(1, 2, 3)` - tuple newtype tag
    UnevaluatedTag {
        position: AnnotationPosition,
        path: Path,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },

    /// Evaluated tag annotation with its computed static value.
    /// Created during the elaborate phase after type analysis.
    Tag {
        position: AnnotationPosition,
        value: Box<StaticExpression>,
    },

    /// Decorator annotation (like `@foo`, `@foo()`, or `@obj.method(args)`).
    ///
    /// Decorators have the same structure as Call expressions - `left` can be any
    /// expression (Path, MemberAccess, etc.) and `arguments` are the decorator's
    /// own arguments (for factory patterns).
    ///
    /// Semantics at runtime:
    /// - `@foo` (no args) → `foo(target)`
    /// - `@foo(x)` (with args) → `foo(x)(target)` (factory pattern)
    /// - `@obj.method` → `obj.method(ta rget)`
    ///
    /// Examples:
    /// - `@memoize` - simple decorator
    /// - `@route("/api")` - decorator factory
    /// - `@service.middleware` - member access decorator
    Decorator {
        position: AnnotationPosition,
        left: LocalNodeId<Expression>,
        arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;

    fn is_evaluated(&self) -> bool {
        match self {
            Annotation::UnevaluatedTag { .. } => false,
            Annotation::Tag { value, .. } => value.is_evaluated(),
            _ => true,
        }
    }
}

impl Annotation {
    /// Get the position of the annotation.
    pub fn position(&self) -> AnnotationPosition {
        match self {
            Annotation::Doc { position, .. } => *position,
            Annotation::Comment { position, .. } => *position,
            Annotation::UnevaluatedTag { position, .. } => *position,
            Annotation::Tag { position, .. } => *position,
            Annotation::Decorator { position, .. } => *position,
        }
    }
}
