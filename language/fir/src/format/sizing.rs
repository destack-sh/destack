use std::iter::FusedIterator;
use std::num::NonZeroU32;
use std::ops::Deref;

use destack_unicode::UnicodeWidthChar;

use super::label::LabelId;
use super::node::{FormatNode, Interned};
use super::tag::{FormatTag, FormatTagKind};

/// Mode used to determine if any variant (except the most expanded) fits for [`BestFittingVariants`].
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum BestFittingMode {
    /// The variant fits if the content up to the first hard or a soft line break inside a [`Group`] with [`PrintMode::Expanded`] fits on the line.
    /// The default mode.
    ///
    /// [`Group`]: tag::Group
    #[default]
    FirstLine,

    /// A variant fits if all lines fit into the configured print width.
    /// A line ends if by any hard or a soft line break inside a [`Group`] with [`PrintMode::Expanded`].
    /// The content doesn't fit if there's any hard line break outside a [`Group`] with [`PrintMode::Expanded`] (a hard line break in content that should be considered in [`PrintMode::Flat`].
    ///
    /// Use this mode with caution as it requires measuring all content of the variant which is more expensive than using [`BestFittingMode::FirstLine`].
    ///
    /// [`Group`]: tag::Group
    AllLines,
}

/// The different variants for this format node.
/// The first node is the one that takes up the most space horizontally (the most flat).
/// The last node takes up the least space horizontally (but most horizontal space).
#[derive(Clone, PartialEq, Debug)]
pub struct BestFittingVariants(Box<[Interned]>);

impl BestFittingVariants {
    /// Create a new best fitting IR with the given variants.
    ///
    /// Callers are required to ensure that the number of variants given is at least 2 when using `most_expanded` or `most_flag`.
    /// You're looking for a way to create a `BestFitting` object, use the `best_fitting![least_expanded, most_expanded]` macro.
    #[doc(hidden)]
    pub fn from_vec_unchecked(variants: Vec<Interned>) -> Self {
        debug_assert!(
            variants.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        Self(variants.into_boxed_slice())
    }

    /// Get the most expanded variant.
    ///
    /// # Panics
    ///
    /// When the number of variants is less than two.
    pub fn most_expanded(&self) -> &[FormatNode] {
        assert!(
            self.0.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        &self.0[self.0.len() - 1]
    }

    pub fn as_slice(&self) -> &[Interned] {
        &self.0
    }

    /// Get the least expanded variant.
    ///
    /// # Panics
    ///
    /// When the number of variants is less than two.
    pub fn most_flat(&self) -> &[FormatNode] {
        assert!(
            self.0.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        &self.0[0]
    }
}

impl Deref for BestFittingVariants {
    type Target = [Interned];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

#[derive(Debug)]
pub struct BestFittingVariantsIter<'a> {
    nodes: std::slice::Iter<'a, Interned>,
}

impl<'a> IntoIterator for &'a BestFittingVariants {
    type Item = &'a [FormatNode];
    type IntoIter = BestFittingVariantsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BestFittingVariantsIter {
            nodes: self.0.iter(),
        }
    }
}

impl<'a> Iterator for BestFittingVariantsIter<'a> {
    type Item = &'a [FormatNode];

    fn next(&mut self) -> Option<Self::Item> {
        self.nodes.next().map(Deref::deref)
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next_back()
    }
}

impl DoubleEndedIterator for BestFittingVariantsIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.nodes.next_back().map(Deref::deref)
    }
}

impl FusedIterator for BestFittingVariantsIter<'_> {}

pub trait FormatNodes {
    /// Check if this [`FormatNode`] is guaranteed to break across multiple lines by the printer.
    /// This is the case if this format node recursively contains a:
    /// - [`crate::builders::empty_line`] or [`crate::builders::hard_line_break`]
    /// - A token containing '\n'
    ///
    /// Use this with caution, this is only a heuristic and the printer may print the node over multiple lines if this node is part of a group and the group doesn't fit on a single line.
    fn will_break(&self) -> bool;

    /// Check if this [`FormatNode`] directly contains a line that can break in flat mode.
    fn may_directly_break(&self) -> bool;

    /// Return the single-line width when every node in this slice is measurable.
    fn single_line_width(&self) -> Option<u32>;

    /// Check if the node has the given label.
    fn has_label(&self, label: LabelId) -> bool;

    /// Get the start tag of `kind` if:
    /// - the last node is an end tag of `kind`.
    /// - there's a matching start tag in this document (may not be true if this slice is an interned node and the `start` is in the document storing the interned node).
    fn start_tag(&self, kind: FormatTagKind) -> Option<&FormatTag>;

    /// Get the end tag if:
    /// - the last node is an end tag of `kind`
    fn end_tag(&self, kind: FormatTagKind) -> Option<&FormatTag>;
}

impl super::node::LineMode {
    /// Return whether this line always breaks.
    pub const fn will_break(self) -> bool {
        matches!(self, Self::Hard | Self::Empty)
    }
}

impl FormatNodes for FormatNode {
    fn will_break(&self) -> bool {
        match self {
            FormatNode::ExpandParent => true,
            FormatNode::Line(line_mode) => line_mode.will_break(),
            FormatNode::Text { width, .. } | FormatNode::FileSlice { width, .. } => {
                width.is_multiline()
            }
            FormatNode::Interned(interned) => interned.will_break(),
            FormatNode::BestFitting { variants, .. } => variants.most_flat().will_break(),
            FormatNode::Tag(FormatTag::StartGroup(group)) => !group.mode().is_flat(),
            FormatNode::Tag(FormatTag::StartConditionalGroup(group)) => !group.mode().is_flat(),
            FormatNode::Space
            | FormatNode::Token { .. }
            | FormatNode::SourcePosition { .. }
            | FormatNode::LineSuffixBoundary
            | FormatNode::Tag(_) => false,
        }
    }

    fn may_directly_break(&self) -> bool {
        match self {
            FormatNode::Line(_) => true,
            FormatNode::Text { width, .. } | FormatNode::FileSlice { width, .. } => {
                width.is_multiline()
            }
            FormatNode::Interned(interned) => interned.may_directly_break(),
            FormatNode::BestFitting { variants, .. } => variants.most_flat().may_directly_break(),
            FormatNode::ExpandParent
            | FormatNode::Space
            | FormatNode::Token { .. }
            | FormatNode::SourcePosition { .. }
            | FormatNode::LineSuffixBoundary
            | FormatNode::Tag(_) => false,
        }
    }

    fn single_line_width(&self) -> Option<u32> {
        match self {
            FormatNode::Space => Some(1),
            FormatNode::Token { text } => Some(text.len() as u32),
            FormatNode::Text { width, .. } | FormatNode::FileSlice { width, .. } => {
                Some(width.width()?.value())
            }
            FormatNode::Interned(interned) => interned.single_line_width(),
            FormatNode::BestFitting { variants, .. } => variants.most_flat().single_line_width(),
            FormatNode::Line(super::node::LineMode::SoftOrSpace) => Some(1),
            FormatNode::Line(_)
            | FormatNode::ExpandParent
            | FormatNode::SourcePosition { .. }
            | FormatNode::LineSuffixBoundary
            | FormatNode::Tag(_) => None,
        }
    }

    fn has_label(&self, _label: LabelId) -> bool {
        false
    }

    fn start_tag(&self, kind: FormatTagKind) -> Option<&FormatTag> {
        match self {
            FormatNode::Tag(tag) if tag.kind() == kind && tag.is_start() => Some(tag),
            _ => None,
        }
    }

    fn end_tag(&self, kind: FormatTagKind) -> Option<&FormatTag> {
        match self {
            FormatNode::Tag(tag) if tag.kind() == kind && tag.is_end() => Some(tag),
            _ => None,
        }
    }
}

impl FormatNodes for [FormatNode] {
    fn will_break(&self) -> bool {
        let mut ignore_line_suffix_depth = 0usize;

        for node in self {
            match node {
                FormatNode::Tag(FormatTag::StartLineSuffix) => {
                    ignore_line_suffix_depth += 1;
                }
                FormatNode::Tag(FormatTag::EndLineSuffix) => {
                    ignore_line_suffix_depth = ignore_line_suffix_depth.saturating_sub(1);
                }
                FormatNode::Interned(interned) if ignore_line_suffix_depth == 0 => {
                    if interned.will_break() {
                        return true;
                    }
                }
                FormatNode::Line(line_mode) if line_mode.will_break() => {
                    return true;
                }
                node if ignore_line_suffix_depth == 0 && node.will_break() => {
                    return true;
                }
                _ => {}
            }
        }

        debug_assert_eq!(ignore_line_suffix_depth, 0, "unclosed line postfix");

        false
    }

    fn may_directly_break(&self) -> bool {
        let mut ignore_line_suffix_depth = 0usize;

        for node in self {
            match node {
                FormatNode::Tag(FormatTag::StartLineSuffix) => {
                    ignore_line_suffix_depth += 1;
                }
                FormatNode::Tag(FormatTag::EndLineSuffix) => {
                    ignore_line_suffix_depth = ignore_line_suffix_depth.saturating_sub(1);
                }
                FormatNode::Interned(interned) if ignore_line_suffix_depth == 0 => {
                    if interned.may_directly_break() {
                        return true;
                    }
                }
                node if ignore_line_suffix_depth == 0 && node.may_directly_break() => {
                    return true;
                }
                _ => {}
            }
        }

        debug_assert_eq!(ignore_line_suffix_depth, 0, "unclosed line postfix");

        false
    }

    fn single_line_width(&self) -> Option<u32> {
        let mut width = 0u32;

        for node in self {
            width = width.saturating_add(node.single_line_width()?);
        }

        Some(width)
    }

    fn has_label(&self, label: LabelId) -> bool {
        self.iter().any(|node| node.has_label(label))
    }

    fn start_tag(&self, kind: FormatTagKind) -> Option<&FormatTag> {
        self.first().and_then(|node| node.start_tag(kind))
    }

    fn end_tag(&self, kind: FormatTagKind) -> Option<&FormatTag> {
        self.last().and_then(|node| node.end_tag(kind))
    }
}

/// Primitives with a textual length that can be passed to [`TextSize::of`].
pub trait TextLen: Copy {
    /// Get the textual length of this primitive.
    fn text_len(self) -> u32;
}

impl TextLen for &'_ str {
    #[inline]
    fn text_len(self) -> u32 {
        self.len().try_into().unwrap()
    }
}

impl TextLen for &'_ String {
    #[inline]
    fn text_len(self) -> u32 {
        self.as_str().text_len()
    }
}

impl TextLen for char {
    #[inline]
    #[expect(clippy::cast_possible_truncation)]
    fn text_len(self) -> u32 {
        self.len_utf8() as u32
    }
}

pub(crate) trait CharWidth {
    fn width(self) -> u8;
}

impl CharWidth for char {
    #[inline]
    fn width(self) -> u8 {
        self.terminal_display_width()
    }
}

/// New-type wrapper for a single-line text unicode width.
/// Mainly to prevent access to the inner value.
///
/// ## Representation
///
/// Represents the width by adding 1 to the actual width so that the width can be represented by a [`NonZeroU32`], allowing [`TextWidth`] or [`Option<Width>`] fit in 4 bytes rather than 8.
///
/// This means that 2^32 can not be precisely represented and instead has the same value as 2^32-1.
/// This imprecision shouldn't matter in practice because either text are longer than any configured line width and thus, the text should break.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Width(NonZeroU32);

impl Width {
    pub(crate) const fn new(width: u32) -> Self {
        Width(NonZeroU32::MIN.saturating_add(width))
    }

    pub const fn value(self) -> u32 {
        self.0.get() - 1
    }
}

/// The pre-computed unicode width of a text if it is a single-line text or a marker that it is a multiline text if it contains a line feed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TextWidth {
    Width(Width),
    Multiline,
}

impl TextWidth {
    pub fn from_text(text: &str, indent_width: u8) -> TextWidth {
        let mut width = 0u32;

        for c in text.chars() {
            let char_width = match c {
                '\t' => indent_width,
                '\n' => return TextWidth::Multiline,
                c => c.width(),
            };
            width += char_width as u32;
        }

        Self::Width(Width::new(width))
    }

    pub const fn width(self) -> Option<Width> {
        match self {
            TextWidth::Width(width) => Some(width),
            TextWidth::Multiline => None,
        }
    }

    pub(crate) const fn is_multiline(self) -> bool {
        matches!(self, TextWidth::Multiline)
    }
}
