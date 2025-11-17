use std::fmt::Debug;

use crate::{Argument, Node, NodeId, NodeType, Path, StringId};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AnnotationPosition {
    /// Annotation inside the node (without next node to attach to, like in an empty block.)
    BlockInfix,
    /// Annotation preceding the node on previous lines (most common).
    BlockPrefix,
    /// Annotation after the node on a following line (only if prefix and infix are not possible).
    BlockPostfix,
    /// Annotation before the node on the same line (like infix comments).
    LinePrefix,
    /// Annotation after the node on the same line (like infix comments).
    LinePostfix,
    /// Annotations after the node on the same line with nothing after it.
    LinePostfixBoundary,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Annotation {
    Blank {
        node: NodeId<Blank>,
        position: AnnotationPosition,
    },
    /// A doc annotation (like `///` or `/**`).
    Doc {
        node: NodeId<Doc>,
        position: AnnotationPosition,
    },
    /// A comment annotation (like `//` or `/*`).
    Comment {
        node: NodeId<Comment>,
        position: AnnotationPosition,
    },
    /// A tag annotation (like `#Foo` or `#Foo(x: 1)`).
    Tag {
        node: NodeId<Tag>,
        position: AnnotationPosition,
    },
    /// A decorator annotation (like `@foo` or `@foo(1, 2, 3)`).
    Decorator {
        node: NodeId<Decorator>,
        position: AnnotationPosition,
    },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;
}

impl Annotation {
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

/// A Blank is a newline or special whitespace.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Blank {
    /// The number of blank lines.
    pub lines: u32,
}

impl Node for Blank {
    const TYPE: NodeType = NodeType::Blank;
}

/// A DocStyle is the style of a documentation comment.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DocStyle {
    /// End of line comment.
    Slash,
    /// Star delimited comment.
    Star,
}

/// A Doc is a block or line-scoped documentation comment string.
/// Like comments, Docs are attached in a side tree outside of the main parse / tree.
///
/// Examples:
/// ```
/// /// Documentation comment.
/// /// Other documentation comment.
/// /**
///  * Documentation comment.
///  */
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    /// The clean documentation comment string.
    /// Newlines preserved, leading/trailing whitespace stripped.
    pub string: StringId,
    /// The style of the documentation comment.
    pub style: DocStyle,
}

impl Node for Doc {
    const TYPE: NodeType = NodeType::Doc;
}

/// A CommentStyle is the style of a comment.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CommentStyle {
    /// End of line comment.
    Slash,
    /// Star delimited comment.
    Star,
}

/// A Comment is a block or line-scoped free-floating comment.
/// Like documentation, Comments are attached in a side tree outside of the main parse / tree.
///
/// Examples:
/// ```
/// // comment
/// /* comment */
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    /// The clean comment string.
    pub string: StringId,
    /// The style of the comment.
    pub style: CommentStyle,
}

impl Node for Comment {
    const TYPE: NodeType = NodeType::Comment;
}

/// A Tag is a block or line-scoped tag annotation.
///
/// Examples:
/// ```
/// #Foo
/// #Foo(x: 1)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    /// The tag name / path.
    pub left: Path,
    /// The arguments (if any).
    pub arguments: Option<Vec<NodeId<Argument>>>,
}

impl Node for Tag {
    const TYPE: NodeType = NodeType::Tag;
}

/// A Decorator is a block-scoped decorator annotation.
/// It looks like a macro call and is prefixed to a block.
///
/// Examples:
/// ```
/// @foo
/// @foo(1, 2, 3)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Decorator {
    /// The decorator name / path.
    pub left: Path,
    /// The arguments (if any).
    pub arguments: Option<Vec<NodeId<Argument>>>,
}

impl Node for Decorator {
    const TYPE: NodeType = NodeType::Decorator;
}
