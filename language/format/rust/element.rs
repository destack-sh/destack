use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use dyst_language_source::Span;

use crate::{BestFittingMode, BestFittingVariants, FormatTag, FormatTagKind, TextWidth};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LineMode {
    Soft,
    SoftOrSpace,
    Hard,
    Empty,
}

/// Language agnostic IR for formatting source code.
///
/// Use the helper functions like [`crate::builders::space`], [`crate::builders::soft_line_break`] etc. defined in this file to create elements.
#[derive(Clone, PartialEq)]
pub enum FormatElement {
    /// A space token, see [`crate::builders::space`] for documentation.
    Space,

    /// A new line, see [`crate::builders::soft_line_break`], [`crate::builders::hard_line_break`], and [`crate::builders::soft_line_break_or_space`] for documentation.
    Line(LineMode),

    /// Forces the parent group to print in expanded mode.
    ExpandParent,

    /// A ASCII only Token that contains no line breaks or tab characters.
    Token { text: &'static str },

    /// An arbitrary text that can contain tabs, newlines, and unicode characters.
    Text {
        /// There's no need for the text to be mutable, using `Box<str>` safes 8 bytes over `String`.
        text: Box<str>,
        text_width: TextWidth,
    },

    /// Text that gets emitted as it is in the source code. Optimized to avoid any allocations.
    Source { slice: Span, text_width: TextWidth },

    /// Prevents that line suffixes move past this boundary. Forces the printer to print any pending
    /// line suffixes, potentially by inserting a hard line break.
    LineBoundary,

    /// An interned format element. Useful when the same content must be emitted multiple times to avoid
    /// deep cloning the IR when using the `best_fitting!` macro or `if_group_fits_on_line` and `if_group_breaks`.
    Interned(Interned),

    /// A list of different variants representing the same content. The printer picks the best fitting content.
    /// Line breaks inside of a best fitting don't propagate to parent groups.
    BestFitting {
        variants: BestFittingVariants,
        mode: BestFittingMode,
    },

    /// A [Tag] that marks the start/end of some content to which some special formatting is applied.
    Tag(FormatTag),
}

impl FormatElement {
    pub fn tag_kind(&self) -> Option<FormatTagKind> {
        if let FormatElement::Tag(tag) = self {
            Some(tag.kind())
        } else {
            None
        }
    }
}

impl std::fmt::Debug for FormatElement {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FormatElement::Space => write!(fmt, "Space"),
            FormatElement::Line(mode) => fmt.debug_tuple("Line").field(mode).finish(),
            FormatElement::ExpandParent => write!(fmt, "ExpandParent"),
            FormatElement::Token { text } => fmt.debug_tuple("Token").field(text).finish(),
            FormatElement::Text { text, .. } => fmt.debug_tuple("DynamicText").field(text).finish(),
            FormatElement::Source { slice, text_width } => fmt
                .debug_tuple("Text")
                .field(slice)
                .field(text_width)
                .finish(),
            FormatElement::LineBoundary => write!(fmt, "LineBoundary"),
            FormatElement::BestFitting { variants, mode } => fmt
                .debug_struct("BestFitting")
                .field("variants", variants)
                .field("mode", &mode)
                .finish(),
            FormatElement::Interned(interned) => fmt.debug_list().entries(&**interned).finish(),
            FormatElement::Tag(tag) => fmt.debug_tuple("Tag").field(tag).finish(),
        }
    }
}

#[derive(Clone)]
pub struct Interned(Rc<[FormatElement]>);

impl Interned {
    pub(super) fn new(content: Vec<FormatElement>) -> Self {
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
    type Target = [FormatElement];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
