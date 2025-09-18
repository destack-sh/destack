use std::collections::HashMap;
use std::ops::Deref;

use crate::{FitsExpanded, FormatElement, FormatTag, Interned, LineMode, group};

/// A formatted document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    elements: Vec<FormatElement>,
}

impl Document {
    /// Sets [`expand`](tag::Group::expand) to [`GroupMode::Propagated`] if the group contains any of:
    /// - a group with [`expand`](tag::Group::expand) set to [`GroupMode::Propagated`] or [`GroupMode::Expand`].
    /// - a non-soft [line break](FormatElement::Line) with mode [`LineMode::Hard`], [`LineMode::Empty`], or [`LineMode::Literal`].
    /// - a [`FormatElement::ExpandParent`]
    ///
    /// [`BestFitting`] elements act as expand boundaries, meaning that the fact that a
    /// [`BestFitting`]'s content expands is not propagated past the [`BestFitting`] element.
    ///
    /// [`BestFitting`]: FormatElement::BestFitting
    pub(crate) fn propagate_expand(&mut self) {
        #[derive(Debug)]
        enum Enclosing<'a> {
            Group(&'a group::Group),
            ConditionalGroup(&'a group::ConditionalGroup),
            FitsExpanded {
                tag: &'a FitsExpanded,
                expands_before: bool,
            },
            BestFitting,
            BestFitParenthesize {
                expanded: bool,
            },
        }

        fn expand_parent(enclosing: &[Enclosing<'_>]) {
            match enclosing.last() {
                Some(Enclosing::Group(group)) => group.propagate_expand(),
                Some(Enclosing::ConditionalGroup(group)) => group.propagate_expand(),
                Some(Enclosing::FitsExpanded { tag, .. }) => tag.propagate_expand(),
                _ => {}
            }
        }

        fn propagate_expands<'a>(
            elements: &'a [FormatElement],
            enclosing: &mut Vec<Enclosing<'a>>,
            checked_interned: &mut HashMap<&'a Interned, bool>,
        ) -> bool {
            let mut expands = false;
            for element in elements {
                let element_expands = match element {
                    FormatElement::Tag(FormatTag::StartGroup(group)) => {
                        enclosing.push(Enclosing::Group(group));
                        false
                    }
                    FormatElement::Tag(FormatTag::EndGroup) => match enclosing.pop() {
                        Some(Enclosing::Group(group)) => !group.mode().is_flat(),
                        _ => false,
                    },
                    FormatElement::Tag(FormatTag::StartBestFitParenthesize { .. }) => {
                        enclosing.push(Enclosing::BestFitParenthesize { expanded: expands });
                        expands = false;
                        continue;
                    }

                    FormatElement::Tag(FormatTag::EndBestFitParenthesize) => {
                        if let Some(Enclosing::BestFitParenthesize { expanded }) = enclosing.pop() {
                            expands = expanded;
                        }
                        continue;
                    }
                    FormatElement::Tag(FormatTag::StartConditionalGroup(group)) => {
                        enclosing.push(Enclosing::ConditionalGroup(group));
                        false
                    }
                    FormatElement::Tag(FormatTag::EndConditionalGroup) => match enclosing.pop() {
                        Some(Enclosing::ConditionalGroup(group)) => !group.mode().is_flat(),
                        _ => false,
                    },
                    FormatElement::Interned(interned) => {
                        if let Some(interned_expands) = checked_interned.get(interned) {
                            *interned_expands
                        } else {
                            let interned_expands =
                                propagate_expands(interned, enclosing, checked_interned);
                            checked_interned.insert(interned, interned_expands);
                            interned_expands
                        }
                    }
                    FormatElement::BestFitting { variants, mode: _ } => {
                        enclosing.push(Enclosing::BestFitting);

                        propagate_expands(variants, enclosing, checked_interned);
                        enclosing.pop();
                        continue;
                    }
                    FormatElement::Tag(FormatTag::StartFitsExpanded(fits_expanded)) => {
                        enclosing.push(Enclosing::FitsExpanded {
                            tag: fits_expanded,
                            expands_before: expands,
                        });
                        false
                    }
                    FormatElement::Tag(FormatTag::EndFitsExpanded) => {
                        if let Some(Enclosing::FitsExpanded { expands_before, .. }) =
                            enclosing.pop()
                        {
                            expands = expands_before;
                        }

                        continue;
                    }
                    FormatElement::Text {
                        text: _,
                        text_width,
                    } => text_width.is_multiline(),
                    FormatElement::SourceSlice { text_width, .. } => text_width.is_multiline(),
                    FormatElement::ExpandParent
                    | FormatElement::Line(LineMode::Hard | LineMode::Empty) => true,
                    _ => false,
                };

                if element_expands {
                    expands = true;
                    expand_parent(enclosing);
                }
            }

            expands
        }

        let mut enclosing = Vec::with_capacity(if self.is_empty() {
            0
        } else {
            self.len().ilog2() as usize
        });
        let mut interned = HashMap::default();
        propagate_expands(self, &mut enclosing, &mut interned);
    }
}

impl From<Vec<FormatElement>> for Document {
    fn from(elements: Vec<FormatElement>) -> Self {
        Self { elements }
    }
}

impl Deref for Document {
    type Target = [FormatElement];

    fn deref(&self) -> &Self::Target {
        self.elements.as_slice()
    }
}
