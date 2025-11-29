use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use destack_source::Span;

use crate::format::{BestFittingMode, BestFittingVariants, FormatTag, FormatTagKind, TextWidth};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LineMode {
    /// Linebreak only if the enclosing group doesn't fit on a single line.
    Soft,
    /// Linebreak only if the enclosing group doesn't fit on a single line, a space otherwise.
    SoftOrSpace,
    /// Linebreak (forced).
    Hard,
    /// Empty line (forced).
    Empty,
}

/// Language agnostic IR for formatting source code.
///
/// Use the helper functions like [`crate::builders::space`], [`crate::builders::soft_line_break`] etc. defined in this file to create nodes.
#[derive(Clone, PartialEq)]
pub enum FormatNode {
    /// A space token, see [`crate::builders::space`] for documentation.
    Space,
    /// Newline, see [`crate::builders::soft_line_break`], [`crate::builders::hard_line_break`], and [`crate::builders::soft_line_break_or_space`] for documentation.
    Line(LineMode),
    /// Forces the parent group to print in expanded mode.
    ExpandParent,
    /// An ASCII Token that contains no line breaks or tab characters.
    Token { text: &'static str },
    /// An arbitrary text that can contain tabs, newlines, and unicode characters.
    Text { text: Box<str>, width: TextWidth },
    /// Text that gets emitted as it is in the source code.
    FileSlice { slice: Span, width: TextWidth },
    /// Prevents that line suffixes move past this boundary.
    /// Forces the printer to print any pending line suffixes, potentially by inserting a hard line break.
    LinePostfixBoundary,
    /// Interned format node.
    /// Useful when the same content must be emitted multiple times to avoid deep cloning the IR when using the `best_fitting!` macro or `if_group_fits_on_line` and `if_group_breaks`.
    Interned(Interned),
    /// List of different variants representing the same content.
    /// The printer picks the best fitting content.
    /// Line breaks inside of a best fitting don't propagate to parent groups.
    BestFitting {
        variants: BestFittingVariants,
        mode: BestFittingMode,
    },
    /// [Tag]s mark the start/end of some content to which some special formatting is applied.
    Tag(FormatTag),
}

impl FormatNode {
    /// Gets the tag kind if this node is a Tag.
    pub fn tag_kind(&self) -> Option<FormatTagKind> {
        if let FormatNode::Tag(tag) = self {
            Some(tag.kind())
        } else {
            None
        }
    }
}

impl std::fmt::Debug for FormatNode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatNode::Space => write!(fmt, "Space"),
            FormatNode::Line(mode) => fmt.debug_tuple("Line").field(mode).finish(),
            FormatNode::ExpandParent => write!(fmt, "ExpandParent"),
            FormatNode::Token { text } => fmt.debug_tuple("Token").field(text).finish(),
            FormatNode::Text { text, .. } => fmt.debug_tuple("DynamicText").field(text).finish(),
            FormatNode::FileSlice {
                slice,
                width: text_width,
            } => fmt
                .debug_tuple("Text")
                .field(slice)
                .field(text_width)
                .finish(),
            FormatNode::LinePostfixBoundary => write!(fmt, "LinePostfixBoundary"),
            FormatNode::BestFitting { variants, mode } => fmt
                .debug_struct("BestFitting")
                .field("variants", variants)
                .field("mode", &mode)
                .finish(),
            FormatNode::Interned(interned) => fmt.debug_list().entries(&**interned).finish(),
            FormatNode::Tag(tag) => fmt.debug_tuple("Tag").field(tag).finish(),
        }
    }
}

/// Interned format node.
#[derive(Clone)]
pub struct Interned(Rc<[FormatNode]>);

impl Interned {
    /// Creates a new Interned from a vector of FormatNodes.
    pub(super) fn new(content: Vec<FormatNode>) -> Self {
        Self(content.into())
    }
}

impl PartialEq for Interned {
    fn eq(&self, other: &Interned) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Interned {}

impl Hash for Interned {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        Rc::as_ptr(&self.0).hash(hasher);
    }
}

impl std::fmt::Debug for Interned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for Interned {
    type Target = [FormatNode];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
