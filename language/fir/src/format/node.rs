use std::ops::Deref;

use destack_source::ByteRange;

use crate::format::{
    ArenaVec, BestFittingMode, BestFittingVariants, FormatTag, FormatTagKind, TextWidth,
};

const _: () = assert!(!std::mem::needs_drop::<FormatNode<'_>>());

#[cfg(all(target_pointer_width = "64", debug_assertions))]
const _: () = assert!(std::mem::size_of::<FormatNode<'_>>() == 48);

#[cfg(all(target_pointer_width = "64", not(debug_assertions)))]
const _: () = assert!(std::mem::size_of::<FormatNode<'_>>() == 24);

/// The printing behavior of one line node.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LineMode {
    /// Break when the enclosing group expands.
    Soft,
    /// Break when the enclosing group expands, or print one space otherwise.
    SoftOrSpace,
    /// Always break the line.
    Hard,
    /// Always print one empty line.
    Empty,
}

/// One language-independent formatting instruction.
#[derive(Clone, PartialEq)]
pub enum FormatNode<'a> {
    /// One space.
    Space,
    /// One conditional or unconditional line break.
    Line(LineMode),
    /// Force the enclosing group to expand.
    ExpandParent,
    /// Static ASCII text without line breaks or tabs.
    Token { text: &'static str },
    /// Borrowed text with precomputed display width.
    Text { text: &'a str, width: TextWidth },
    /// The source position for subsequent output.
    SourcePosition { source: u32 },
    /// One verbatim file-local source range.
    FileSlice { range: ByteRange, width: TextWidth },
    /// A boundary that flushes pending line suffixes.
    LineSuffixBoundary,
    /// One reusable arena-backed node slice.
    Slice(NodeSlice<'a>),
    /// Alternative layouts ordered from most flat to most expanded.
    BestFitting {
        /// The available layouts.
        variants: BestFittingVariants<'a>,
        /// The measurement policy used to choose a layout.
        mode: BestFittingMode,
    },
    /// One structural formatting tag.
    Tag(FormatTag),
}

impl FormatNode<'_> {
    /// Return this node's tag kind when present.
    pub fn tag_kind(&self) -> Option<FormatTagKind> {
        if let FormatNode::Tag(tag) = self {
            Some(tag.kind())
        } else {
            None
        }
    }
}

impl<'a> ArenaVec<'a, FormatNode<'a>> {
    /// Collapse this sequence into one optional node.
    pub fn collapse(mut self) -> Option<FormatNode<'a>> {
        match self.len() {
            0 => None,
            1 => self.pop(),
            _ => Some(FormatNode::Slice(NodeSlice::new(self))),
        }
    }
}

impl std::fmt::Debug for FormatNode<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatNode::Space => write!(fmt, "Space"),
            FormatNode::Line(mode) => fmt.debug_tuple("Line").field(mode).finish(),
            FormatNode::ExpandParent => write!(fmt, "ExpandParent"),
            FormatNode::Token { text } => fmt.debug_tuple("Token").field(text).finish(),
            FormatNode::Text { text, .. } => fmt.debug_tuple("Text").field(text).finish(),
            FormatNode::SourcePosition { source } => {
                fmt.debug_tuple("SourcePosition").field(source).finish()
            }
            FormatNode::FileSlice {
                range,
                width: text_width,
            } => fmt
                .debug_tuple("FileSlice")
                .field(range)
                .field(text_width)
                .finish(),
            FormatNode::LineSuffixBoundary => write!(fmt, "LineSuffixBoundary"),
            FormatNode::BestFitting { variants, mode } => fmt
                .debug_struct("BestFitting")
                .field("variants", variants)
                .field("mode", &mode)
                .finish(),
            FormatNode::Slice(slice) => fmt.debug_list().entries(&**slice).finish(),
            FormatNode::Tag(tag) => fmt.debug_tuple("Tag").field(tag).finish(),
        }
    }
}

/// One reusable arena-backed FIR node slice.
#[derive(Clone, Copy)]
pub struct NodeSlice<'a>(&'a [FormatNode<'a>]);

impl<'a> NodeSlice<'a> {
    /// Store one node vector as a stable slice.
    pub(super) fn new(content: ArenaVec<'a, FormatNode<'a>>) -> Self {
        Self(content.into_slice())
    }

    /// Return the pointer identity for this node slice.
    pub(super) fn as_ptr(&self) -> *const FormatNode<'a> {
        self.0.as_ptr()
    }
}

impl PartialEq for NodeSlice<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for NodeSlice<'_> {}

impl std::fmt::Debug for NodeSlice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<'a> Deref for NodeSlice<'a> {
    type Target = [FormatNode<'a>];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
