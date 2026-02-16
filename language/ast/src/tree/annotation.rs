use std::fmt::Debug;

use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Node, NodeType, StringId};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Annotation {
    /// A doc annotation (like `///` or `/**`).
    Doc {
        node: LocalNodeId<Doc>,
        position: AnnotationPosition,
    },
    /// A decorator annotation (like `@foo` or `@foo(1, 2, 3)`).
    Decorator {
        node: LocalNodeId<Decorator>,
        position: AnnotationPosition,
    },
}

impl Node for Annotation {
    const TYPE: NodeType = NodeType::Annotation;
}

impl Annotation {
    pub fn position(&self) -> AnnotationPosition {
        match self {
            Annotation::Doc { position, .. } => *position,
            Annotation::Decorator { position, .. } => *position,
        }
    }
}

/// A Blank is a newline or special whitespace.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blank {
    /// The number of blank lines.
    pub lines: u32,
}

impl Node for Blank {
    const TYPE: NodeType = NodeType::Blank;
}

/// The style of a documentation comment.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum DocumentationStyle {
    /// End of line comment.
    Slash,
    /// Star delimited comment.
    Star,
}

/// A block or line-scoped documentation comment string.
///
/// Examples:
/// ```
/// /// Documentation comment.
/// /// Other documentation comment.
/// /**
///  * Documentation comment.
///  */
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Documentation {
    /// The clean documentation comment string.
    /// Newlines preserved, leading/trailing whitespace stripped.
    pub string: StringId,
    /// The style of the documentation comment.
    pub style: DocumentationStyle,
}

impl Node for Documentation {
    const TYPE: NodeType = NodeType::Doc;
}

/// Compatibility alias for transition from `DocStyle`.
pub type DocStyle = DocumentationStyle;

/// Compatibility alias for transition from `Doc`.
pub type Doc = Documentation;

/// The style of a comment.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    /// The clean comment string.
    pub string: StringId,
    /// The style of the comment.
    pub style: CommentStyle,
}

impl Node for Comment {
    const TYPE: NodeType = NodeType::Comment;
}

/// A normalized directive extracted from a comment in the lexer.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CommentDirective {
    /// No recognized directive marker.
    #[default]
    None = 0,
    /// A legal header marker.
    Legal = 1,
    /// A `__PURE__` style marker.
    Pure = 2,
    /// A `#__NO_SIDE_EFFECTS__` style marker.
    NoSideEffects = 3,
    /// A TypeScript line directive marker.
    TypeScript = 4,
    /// A formatter ignore-next marker.
    FormatIgnore = 5,
    /// A formatter ignore-file directive marker.
    FormatIgnoreFile = 6,
    /// A formatter ignore-range start marker.
    FormatIgnoreStart = 7,
    /// A formatter ignore-range end marker.
    FormatIgnoreEnd = 8,
}

/// Newline shape flags captured around one trivia record.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TriviaNewlineFlags {
    /// Bit flags that describe newline boundaries.
    pub bits: u8,
}

impl TriviaNewlineFlags {
    /// Leading newline bit.
    pub const LEADING: u8 = 1 << 0;
    /// Trailing newline bit.
    pub const TRAILING: u8 = 1 << 1;

    /// Create flags from booleans.
    #[inline]
    pub fn from_bools(has_leading_newline: bool, has_trailing_newline: bool) -> Self {
        let mut bits = 0u8;

        if has_leading_newline {
            bits |= Self::LEADING;
        }
        if has_trailing_newline {
            bits |= Self::TRAILING;
        }

        Self { bits }
    }

    /// Return whether a leading newline exists.
    #[inline]
    pub fn has_leading_newline(self) -> bool {
        self.bits & Self::LEADING != 0
    }

    /// Return whether a trailing newline exists.
    #[inline]
    pub fn has_trailing_newline(self) -> bool {
        self.bits & Self::TRAILING != 0
    }
}

/// Common token-boundary metadata for trivia.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriviaBoundary {
    /// The semantic token before this trivia, or `u32::MAX`.
    pub token_before: u32,
    /// The semantic token after this trivia, or `u32::MAX`.
    pub token_after: u32,
    /// Newline shape around this trivia.
    pub newlines: TriviaNewlineFlags,
    /// Whether this trivia can be consumed as a leading candidate.
    pub is_leading_candidate: bool,
}

/// Comment trivia payload with placement metadata.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentTrivia {
    /// The comment node id.
    pub comment: LocalNodeId<Comment>,
    /// The original source span for this trivia record.
    pub span: Span,
    /// Token-boundary metadata.
    pub boundary: TriviaBoundary,
    /// Normalized lexer directive kind.
    pub directive: CommentDirective,
}

/// Blank trivia payload with placement metadata.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlankTrivia {
    /// The blank node id.
    pub blank: LocalNodeId<Blank>,
    /// The original source span for this trivia record.
    pub span: Span,
    /// Token-boundary metadata.
    pub boundary: TriviaBoundary,
}

/// Stable source-order reference into split trivia buffers.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriviaRef {
    /// Index into `NodeTree::comment_trivia`.
    Comment(u32),
    /// Index into `NodeTree::blank_trivia`.
    Blank(u32),
}

/// A Decorator is a block-scoped decorator annotation.
/// It looks like a macro call and is prefixed to a block.
///
/// Examples:
/// ```
/// @foo
/// @foo(1, 2, 3)
/// @foo<T>()
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decorator {
    /// The decorator expression.
    pub expression: LocalNodeId<Expression>,
}

impl Node for Decorator {
    const TYPE: NodeType = NodeType::Decorator;
}
