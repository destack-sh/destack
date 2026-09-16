use crate::{
    Discriminant, DiscriminantField, Layout, LayoutShape, Representation, Scalar, ScalarField,
    TraceMap, TypeId, Validity, VariantCase, VariantCaseLayout, VariantEncoding, VariantLayout,
    VariantTrace,
};

use super::{LayoutBuilder, LayoutError};

/// Physical construction state for one variant type.
pub(super) struct Variant {
    /// The logical discriminant type.
    discriminant: TypeId,
    /// The physical discriminant layout.
    tag: Tag,
    /// The variant cases in source order.
    cases: Vec<Case>,
    /// The widest payload size.
    payload_size: u32,
    /// The strictest payload alignment.
    payload_alignment: u32,
}

/// Physical properties of one variant discriminant.
struct Tag {
    /// The discriminant register representation.
    representation: Representation,
    /// The discriminant size in bytes.
    size: u32,
    /// The discriminant alignment in bytes.
    alignment: u32,
}

/// One variant case and its computed payload layout.
struct Case {
    /// The logical discriminant bits.
    discriminant: Discriminant,
    /// The logical payload type.
    ty: TypeId,
    /// The payload register representation.
    representation: Representation,
    /// The largest invalid scalar range in the payload.
    niche: Option<ScalarField>,
    /// The payload size in bytes.
    size: u32,
    /// The payload alignment in bytes.
    alignment: u32,
    /// The payload reference trace map.
    trace_map: TraceMap,
    /// Whether the payload admits no value.
    uninhabited: bool,
}

impl Variant {
    /// Compute the case layouts required to construct one variant.
    pub(super) fn new(
        discriminant: TypeId,
        cases: &[VariantCase],
        layouts: &mut LayoutBuilder<'_>,
    ) -> Result<Self, LayoutError> {
        let tag = layouts.layout_type(discriminant)?;
        let tag = layouts.layout(tag);
        let tag = Tag {
            representation: tag.representation,
            size: tag.size,
            alignment: tag.alignment,
        };
        let mut payload_size = 0u32;
        let mut payload_alignment = 1u32;
        let mut computed = Vec::with_capacity(cases.len());

        // compute every logical discriminant and payload layout
        for case in cases {
            let Some(discriminant) = case.discriminant() else {
                return Err(LayoutError::InvalidDiscriminant {
                    constant: format!("{:?}", case.discriminant),
                });
            };
            let layout = layouts.layout_type(case.ty)?;
            let layout = layouts.layout(layout);
            payload_size = payload_size.max(layout.size);
            payload_alignment = payload_alignment.max(layout.alignment);
            computed.push(Case {
                discriminant,
                ty: case.ty,
                representation: layout.representation,
                niche: layout.niche,
                size: layout.size,
                alignment: layout.alignment,
                trace_map: layout.trace_map.clone(),
                uninhabited: layout.uninhabited,
            });
        }

        Ok(Self {
            discriminant,
            tag,
            cases: computed,
            payload_size,
            payload_alignment,
        })
    }

    /// Select the most compact valid encoding for this variant.
    pub(super) fn layout(self) -> Result<Layout, LayoutError> {
        if let Some(layout) = self.niche()? {
            return Ok(layout);
        }

        self.direct()
    }

    /// Encode this variant in one payload niche when possible.
    fn niche(&self) -> Result<Option<Layout>, LayoutError> {
        if self.cases.len() < 2 {
            return Ok(None);
        }
        let inhabited = self.cases.iter().filter(|case| !case.uninhabited).count();

        // require exactly one inhabited case containing physical bytes
        let mut untagged = None;
        for (index, case) in self.cases.iter().enumerate() {
            if case.size == 0 || case.uninhabited {
                continue;
            }
            if untagged.is_some() {
                return Ok(None);
            }
            untagged = Some((index, case));
        }
        let Some((untagged_case, payload)) = untagged else {
            return Ok(None);
        };
        let required = inhabited.saturating_sub(1) as u128;
        let Some(field) = payload.niche else {
            return Ok(None);
        };
        let scalar = field.scalar;
        let Some((niche_start, niche_count)) = scalar.validity.invalid(scalar.bit_width()) else {
            return Ok(None);
        };
        if niche_count < required {
            return Ok(None);
        }

        // admit the encoded cases and retain any remaining invalid values
        let mask = scalar.bit_mask();
        let end = scalar.validity.end.bits().wrapping_add(required) & mask;
        let expanded = Scalar::with_validity(
            scalar.primitive,
            Validity::new(scalar.validity.start.bits(), end),
        );
        let niche = if niche_count == required {
            None
        } else {
            expanded.niche(field.offset)
        };
        let representation = match payload.representation {
            Representation::Scalar(_) if field.offset == 0 => Representation::Scalar(expanded),
            Representation::ScalarPair(mut fields) => {
                let Some(candidate) = fields
                    .iter_mut()
                    .find(|candidate| candidate.offset == field.offset)
                else {
                    return Err(LayoutError::InvalidNiche {
                        offset: field.offset,
                    });
                };
                candidate.scalar = expanded;

                Representation::ScalarPair(fields)
            }
            Representation::Memory => Representation::Memory,
            _ => {
                return Err(LayoutError::InvalidNiche {
                    offset: field.offset,
                });
            }
        };

        // map every encoded case around the untagged payload case
        let cases = self.case_layouts(0);
        let encoding = VariantEncoding::Niche {
            field: DiscriminantField {
                offset: field.offset,
                byte_len: field.scalar.bit_width().div_ceil(8) as u8,
                bit_offset: 0,
                bit_len: field.scalar.bit_width() as u8,
            },
            untagged_case: untagged_case as u32,
            niche_start,
        };
        let trace_map = self.trace(encoding, &cases);

        Ok(Some(Layout {
            shape: LayoutShape::Variant(VariantLayout {
                discriminant: self.discriminant,
                encoding,
                cases,
            }),
            representation,
            niche,
            size: payload.size,
            alignment: payload.alignment,
            trace_map,
            uninhabited: false,
        }))
    }

    /// Store this variant behind one direct discriminant field.
    fn direct(self) -> Result<Layout, LayoutError> {
        let Representation::Scalar(tag_scalar) = self.tag.representation else {
            return Err(LayoutError::Unsupported {
                construct: "a layout for this variant discriminant".to_string(),
            });
        };
        let tag_size = self.tag.size;
        let tag_alignment = self.tag.alignment;

        // retain the tightest wrapping validity range covering every inhabited case tag
        let mask = tag_scalar.bit_mask();
        let mut values = self
            .cases
            .iter()
            .filter(|case| !case.uninhabited)
            .map(|case| case.discriminant.bits() & mask)
            .collect::<Vec<_>>();
        values.sort_unstable();
        values.dedup();
        let tag_scalar = if let Some(&first) = values.first() {
            let mut validity = Validity::new(first, first);
            let mut largest_gap = 0;
            for index in 0..values.len() {
                let current = values[index];
                let next = values.get(index + 1).copied().unwrap_or(first);
                let gap = if current < next {
                    next - current - 1
                } else {
                    mask - current + next
                };
                if gap >= largest_gap {
                    largest_gap = gap;
                    validity = Validity::new(next, current);
                }
            }

            Scalar::with_validity(tag_scalar.primitive, validity)
        } else {
            tag_scalar
        };

        // place the shared payload after the direct tag
        let payload_offset = tag_size.next_multiple_of(self.payload_alignment.max(1));
        let alignment = tag_alignment.max(self.payload_alignment);
        let size = (payload_offset + self.payload_size).next_multiple_of(alignment);
        let cases = self.case_layouts(payload_offset);
        let encoding = VariantEncoding::Direct {
            field: DiscriminantField::scalar(0, tag_size as u8),
        };

        // keep direct payloads in registers only under one shared scalar primitive
        let representation = if self.payload_size == 0 {
            Representation::Scalar(tag_scalar)
        } else {
            let mut payload = None;
            for case in self.cases.iter().filter(|case| case.size != 0) {
                let Representation::Scalar(scalar) = case.representation else {
                    payload = None;
                    break;
                };
                if payload.is_some_and(|primitive| primitive != scalar.primitive) {
                    payload = None;
                    break;
                }
                payload = Some(scalar.primitive);
            }

            match payload {
                Some(payload) => Representation::ScalarPair([
                    ScalarField::new(tag_scalar, 0),
                    ScalarField::new(Scalar::new(payload), payload_offset),
                ]),
                None => Representation::Memory,
            }
        };
        let trace_map = self.trace(encoding, &cases);

        Ok(Layout {
            shape: LayoutShape::Variant(VariantLayout {
                discriminant: self.discriminant,
                encoding,
                cases,
            }),
            representation,
            niche: tag_scalar.niche(0),
            size,
            alignment,
            trace_map,
            uninhabited: self.cases.iter().all(|case| case.uninhabited),
        })
    }

    /// Return the physical case records at one shared payload offset.
    fn case_layouts(&self, payload_offset: u32) -> Vec<VariantCaseLayout> {
        self.cases
            .iter()
            .map(|case| VariantCaseLayout {
                discriminant: case.discriminant,
                ty: case.ty,
                payload_offset,
            })
            .collect()
    }

    /// Build the case-selected trace map for one variant encoding.
    fn trace(&self, encoding: VariantEncoding, cases: &[VariantCaseLayout]) -> TraceMap {
        let cases = cases
            .iter()
            .zip(&self.cases)
            .map(|(layout, case)| VariantTrace {
                discriminant: layout.discriminant,
                payload_offset: layout.payload_offset,
                map: case.trace_map.clone(),
            })
            .collect::<Vec<_>>();
        if cases.iter().all(|case| !case.map.has_reference()) {
            TraceMap::Empty
        } else {
            TraceMap::Variant {
                encoding,
                cases: cases.into_boxed_slice(),
            }
        }
    }
}
