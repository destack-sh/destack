use std::{iter::FusedIterator, num::NonZeroU32, ops::Deref};

use crate::{FormatElement, LabelId, FormatTag, FormatTagKind};


/// Mode used to determine if any variant (except the most expanded) fits for [`BestFittingVariants`].
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum BestFittingMode {
    /// The variant fits if the content up to the first hard or a soft line break inside a [`Group`] with
    /// [`PrintMode::Expanded`] fits on the line. The default mode.
    ///
    /// [`Group`]: tag::Group
    #[default]
    FirstLine,

    /// A variant fits if all lines fit into the configured print width. A line ends if by any
    /// hard or a soft line break inside a [`Group`] with [`PrintMode::Expanded`].
    /// The content doesn't fit if there's any hard line break  outside a [`Group`] with [`PrintMode::Expanded`]
    /// (a hard line break in content that should be considered in [`PrintMode::Flat`].
    ///
    /// Use this mode with caution as it requires measuring all content of the variant which is more
    /// expensive than using [`BestFittingMode::FirstLine`].
    ///
    /// [`Group`]: tag::Group
    AllLines,
}

/// The different variants for this element.
/// The first element is the one that takes up the most space horizontally (the most flat),
/// The last element takes up the least space horizontally (but most horizontal space).
#[derive(Clone, PartialEq, Debug)]
pub struct BestFittingVariants(Box<[FormatElement]>);

impl BestFittingVariants {
    /// Creates a new best fitting IR with the given variants.
    ///
    /// Callers are required to ensure that the number of variants given
    /// is at least 2 when using `most_expanded` or `most_flag`.
    ///
    /// You're looking for a way to create a `BestFitting` object, use the `best_fitting![least_expanded, most_expanded]` macro.
    #[doc(hidden)]
    pub fn from_vec_unchecked(variants: Vec<FormatElement>) -> Self {
        debug_assert!(
            variants
                .iter()
                .filter(|element| matches!(element, FormatElement::Tag(FormatTag::StartBestFittingEntry)))
                .count()
                >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        Self(variants.into_boxed_slice())
    }

    /// Returns the most expanded variant
    ///
    /// # Panics
    ///
    /// When the number of variants is less than two.
    pub fn most_expanded(&self) -> &[FormatElement] {
        assert!(
            self.as_slice()
                .iter()
                .filter(|element| matches!(element, FormatElement::Tag(FormatTag::StartBestFittingEntry)))
                .count()
                >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        self.into_iter().last().unwrap()
    }

    pub fn as_slice(&self) -> &[FormatElement] {
        &self.0
    }

    /// Returns the least expanded variant
    ///
    /// # Panics
    ///
    /// When the number of variants is less than two.
    pub fn most_flat(&self) -> &[FormatElement] {
        assert!(
            self.as_slice()
                .iter()
                .filter(|element| matches!(element, FormatElement::Tag(FormatTag::StartBestFittingEntry)))
                .count()
                >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        self.into_iter().next().unwrap()
    }
}

impl Deref for BestFittingVariants {
    type Target = [FormatElement];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

pub struct BestFittingVariantsIter<'a> {
    elements: &'a [FormatElement],
}

impl<'a> IntoIterator for &'a BestFittingVariants {
    type Item = &'a [FormatElement];
    type IntoIter = BestFittingVariantsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BestFittingVariantsIter { elements: &self.0 }
    }
}

impl<'a> Iterator for BestFittingVariantsIter<'a> {
    type Item = &'a [FormatElement];

    fn next(&mut self) -> Option<Self::Item> {
        match self.elements.first()? {
            FormatElement::Tag(FormatTag::StartBestFittingEntry) => {
                let end = self
                    .elements
                    .iter()
                    .position(|element| {
                        matches!(element, FormatElement::Tag(FormatTag::EndBestFittingEntry))
                    })
                    .map_or(self.elements.len(), |position| position + 1);

                let (variant, rest) = self.elements.split_at(end);
                self.elements = rest;

                Some(variant)
            }
            _ => None,
        }
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
        let start_position = self.elements.iter().rposition(|element| {
            matches!(element, FormatElement::Tag(FormatTag::StartBestFittingEntry))
        })?;

        let (rest, variant) = self.elements.split_at(start_position);
        self.elements = rest;
        Some(variant)
    }
}

impl FusedIterator for BestFittingVariantsIter<'_> {}

pub trait FormatElements {
    /// Returns true if this [`FormatElement`] is guaranteed to break across multiple lines by the printer.
    /// This is the case if this format element recursively contains a:
    /// - [`crate::builders::empty_line`] or [`crate::builders::hard_line_break`]
    /// - A token containing '\n'
    ///
    /// Use this with caution, this is only a heuristic and the printer may print the element over multiple
    /// lines if this element is part of a group and the group doesn't fit on a single line.
    fn will_break(&self) -> bool;

    /// Returns true if the element has the given label.
    fn has_label(&self, label: LabelId) -> bool;

    /// Returns the start tag of `kind` if:
    /// - the last element is an end tag of `kind`.
    /// - there's a matching start tag in this document (may not be true if this slice is an interned element and the `start` is in the document storing the interned element).
    fn start_tag(&self, kind: FormatTagKind) -> Option<&FormatTag>;

    /// Returns the end tag if:
    /// - the last element is an end tag of `kind`
    fn end_tag(&self, kind: FormatTagKind) -> Option<&FormatTag>;
}

/// New-type wrapper for a single-line text unicode width.
/// Mainly to prevent access to the inner value.
///
/// ## Representation
///
/// Represents the width by adding 1 to the actual width so that the width can be represented by a [`NonZeroU32`],
/// allowing [`TextWidth`] or [`Option<Width>`] fit in 4 bytes rather than 8.
///
/// This means that 2^32 can not be precisely represented and instead has the same value as 2^32-1.
/// This imprecision shouldn't matter in practice because either text are longer than any configured line width
/// and thus, the text should break.
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

/// The pre-computed unicode width of a text if it is a single-line text or a marker
/// that it is a multiline text if it contains a line feed.
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
                #[expect(clippy::cast_possible_truncation)]
                c => c.width().unwrap_or(0) as u32,
            };
            width += char_width;
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