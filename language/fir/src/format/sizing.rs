use std::iter::FusedIterator;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::slice;

use destack_unicode::UnicodeWidthChar;

use super::ArenaVec;
use super::label::LabelId;
use super::node::{FormatNode, NodeSlice};
use super::tag::{FormatTag, FormatTagKind};

/// The measurement policy used to select one best-fitting variant.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum BestFittingMode {
    /// Measure content through its first effective line break.
    #[default]
    FirstLine,

    /// Measure every line in the candidate layout.
    AllLines,
}

/// Alternative layouts ordered from most flat to most expanded.
#[derive(Clone, PartialEq, Debug)]
pub struct BestFittingVariants<'a>(&'a [NodeSlice<'a>]);

impl<'a> BestFittingVariants<'a> {
    /// Create best-fitting variants without validating their count in release builds.
    #[doc(hidden)]
    pub fn from_vec_unchecked(variants: ArenaVec<'a, NodeSlice<'a>>) -> Self {
        debug_assert!(
            variants.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        Self(variants.into_slice())
    }

    /// Return the most expanded variant.
    ///
    /// # Panics
    ///
    /// When the number of variants is less than two.
    pub fn most_expanded(&self) -> &[FormatNode<'a>] {
        assert!(
            self.0.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        &self.0[self.0.len() - 1]
    }

    /// Return the variants from most flat to most expanded.
    pub fn as_slice(&self) -> &[NodeSlice<'a>] {
        self.0
    }

    /// Return the most flat variant.
    ///
    /// # Panics
    ///
    /// When the number of variants is less than two.
    pub fn most_flat(&self) -> &[FormatNode<'a>] {
        assert!(
            self.0.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        &self.0[0]
    }
}

impl<'a> Deref for BestFittingVariants<'a> {
    type Target = [NodeSlice<'a>];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

/// An iterator over best-fitting node slices.
#[derive(Debug)]
pub struct BestFittingVariantsIter<'a> {
    nodes: std::slice::Iter<'a, NodeSlice<'a>>,
}

impl<'a> IntoIterator for &'a BestFittingVariants<'a> {
    type Item = &'a [FormatNode<'a>];
    type IntoIter = BestFittingVariantsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BestFittingVariantsIter {
            nodes: self.0.iter(),
        }
    }
}

impl<'a> Iterator for BestFittingVariantsIter<'a> {
    type Item = &'a [FormatNode<'a>];

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

/// Layout queries over one FIR node or node slice.
pub trait FormatNodes {
    /// Return whether this content is guaranteed to break.
    fn will_break(&self) -> bool;

    /// Return whether this content directly contains a breakable line.
    fn may_directly_break(&self) -> bool;

    /// Return the single-line width when every node in this slice is measurable.
    fn single_line_width(&self) -> Option<u32>;

    /// Return whether this content has one label.
    fn has_label(&self, label: LabelId) -> bool;

    /// Return a leading start tag of one kind.
    fn start_tag(&self, kind: FormatTagKind) -> Option<&FormatTag>;

    /// Return a trailing end tag of one kind.
    fn end_tag(&self, kind: FormatTagKind) -> Option<&FormatTag>;
}

impl super::node::LineMode {
    /// Return whether this line always breaks.
    pub const fn will_break(self) -> bool {
        matches!(self, Self::Hard | Self::Empty)
    }
}

impl FormatNodes for FormatNode<'_> {
    fn will_break(&self) -> bool {
        std::slice::from_ref(self).will_break()
    }

    fn may_directly_break(&self) -> bool {
        std::slice::from_ref(self).may_directly_break()
    }

    fn single_line_width(&self) -> Option<u32> {
        std::slice::from_ref(self).single_line_width()
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

impl FormatNodes for [FormatNode<'_>] {
    fn will_break(&self) -> bool {
        let mut ignore_line_suffix_depth = 0usize;

        let mut nodes = NodeTraversal::new(self);
        while let Some(node) = nodes.next() {
            match node {
                FormatNode::Tag(FormatTag::StartLineSuffix) => {
                    ignore_line_suffix_depth += 1;
                }
                FormatNode::Tag(FormatTag::EndLineSuffix) => {
                    ignore_line_suffix_depth = ignore_line_suffix_depth.saturating_sub(1);
                }
                FormatNode::Slice(slice) if ignore_line_suffix_depth == 0 => {
                    nodes.enter(slice);
                }
                FormatNode::BestFitting { variants, .. } if ignore_line_suffix_depth == 0 => {
                    nodes.enter(variants.most_flat());
                }
                FormatNode::Line(line_mode) if line_mode.will_break() => {
                    return true;
                }
                FormatNode::ExpandParent if ignore_line_suffix_depth == 0 => {
                    return true;
                }
                FormatNode::Text { width, .. } | FormatNode::FileSlice { width, .. }
                    if ignore_line_suffix_depth == 0 && width.is_multiline() =>
                {
                    return true;
                }
                FormatNode::Tag(FormatTag::StartGroup(group))
                    if ignore_line_suffix_depth == 0 && !group.mode().is_flat() =>
                {
                    return true;
                }
                FormatNode::Tag(FormatTag::StartConditionalGroup(group))
                    if ignore_line_suffix_depth == 0 && !group.mode().is_flat() =>
                {
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

        let mut nodes = NodeTraversal::new(self);
        while let Some(node) = nodes.next() {
            match node {
                FormatNode::Tag(FormatTag::StartLineSuffix) => {
                    ignore_line_suffix_depth += 1;
                }
                FormatNode::Tag(FormatTag::EndLineSuffix) => {
                    ignore_line_suffix_depth = ignore_line_suffix_depth.saturating_sub(1);
                }
                FormatNode::Slice(slice) if ignore_line_suffix_depth == 0 => {
                    nodes.enter(slice);
                }
                FormatNode::BestFitting { variants, .. } if ignore_line_suffix_depth == 0 => {
                    nodes.enter(variants.most_flat());
                }
                FormatNode::Line(_) if ignore_line_suffix_depth == 0 => {
                    return true;
                }
                FormatNode::Text { width, .. } | FormatNode::FileSlice { width, .. }
                    if ignore_line_suffix_depth == 0 && width.is_multiline() =>
                {
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

        let mut nodes = NodeTraversal::new(self);
        while let Some(node) = nodes.next() {
            let node_width = match node {
                FormatNode::Space | FormatNode::Line(super::node::LineMode::SoftOrSpace) => 1,
                FormatNode::Token { text } => text.len() as u32,
                FormatNode::Text { width, .. } | FormatNode::FileSlice { width, .. } => {
                    width.width()?.value()
                }
                FormatNode::Line(_)
                | FormatNode::ExpandParent
                | FormatNode::SourcePosition { .. }
                | FormatNode::LineSuffixBoundary
                | FormatNode::Tag(_) => return None,
                FormatNode::Slice(slice) => {
                    nodes.enter(slice);
                    continue;
                }
                FormatNode::BestFitting { variants, .. } => {
                    nodes.enter(variants.most_flat());
                    continue;
                }
            };
            width = width.saturating_add(node_width);
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

/// One explicit depth-first traversal over FIR node slices.
struct NodeTraversal<'nodes, 'arena> {
    current: slice::Iter<'nodes, FormatNode<'arena>>,
    parents: Vec<slice::Iter<'nodes, FormatNode<'arena>>>,
}

impl<'nodes, 'arena: 'nodes> NodeTraversal<'nodes, 'arena> {
    /// Create one traversal over a node slice.
    fn new(nodes: &'nodes [FormatNode<'arena>]) -> Self {
        Self {
            current: nodes.iter(),
            parents: Vec::new(),
        }
    }

    /// Enter one nested node slice before continuing the current slice.
    fn enter(&mut self, nodes: &'nodes [FormatNode<'arena>]) {
        let parent = std::mem::replace(&mut self.current, nodes.iter());
        self.parents.push(parent);
    }
    /// Return the next node without implicitly entering structural nodes.
    fn next(&mut self) -> Option<&'nodes FormatNode<'arena>> {
        loop {
            // read or leave the current slice
            let Some(node) = self.current.next() else {
                self.current = self.parents.pop()?;
                continue;
            };

            return Some(node);
        }
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

#[cfg(test)]
mod tests {
    use crate::format::{Allocator, ArenaVec, FormatNode, FormatTag, LineMode, NodeSlice};

    use super::FormatNodes;

    /// Query deeply nested FIR slices without consuming call stack.
    #[test]
    fn test_query_nested_node_slices() {
        let allocator = Allocator::default();
        let mut breaking = FormatNode::Line(LineMode::Hard);
        let mut flat = FormatNode::Token { text: "x" };

        for _ in 0..100_000 {
            let breaking_nodes = ArenaVec::from_array_in([breaking], &allocator);
            breaking = FormatNode::Slice(NodeSlice::new(breaking_nodes));

            let flat_nodes = ArenaVec::from_array_in([flat], &allocator);
            flat = FormatNode::Slice(NodeSlice::new(flat_nodes));
        }

        assert!(breaking.will_break());
        assert!(breaking.may_directly_break());
        assert_eq!(breaking.single_line_width(), None);
        assert!(!flat.will_break());
        assert!(!flat.may_directly_break());
        assert_eq!(flat.single_line_width(), Some(1));
    }

    /// Ignore nested line breaks owned by line suffixes.
    #[test]
    fn test_queries_skip_nested_line_suffix_content() {
        let allocator = Allocator::default();
        let suffix = ArenaVec::from_array_in([FormatNode::Line(LineMode::Hard)], &allocator);
        let suffix = NodeSlice::new(suffix);
        let nodes = [
            FormatNode::Tag(FormatTag::StartLineSuffix),
            FormatNode::Slice(suffix),
            FormatNode::Tag(FormatTag::EndLineSuffix),
        ];

        assert!(!nodes.will_break());
        assert!(!nodes.may_directly_break());
        assert_eq!(nodes.single_line_width(), None);
    }
}
