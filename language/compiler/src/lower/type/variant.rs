use destack_mir as mir;
use destack_source::ModuleId;

use super::{LayoutBuilder, LayoutError};

/// Physical construction state for one variant type.
pub(super) struct Variant {
    /// The logical discriminant type.
    discriminant: mir::TypeId,
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
    representation: mir::Representation,
    /// The discriminant size in bytes.
    size: u32,
    /// The discriminant alignment in bytes.
    alignment: u32,
}

/// One variant case and its computed payload layout.
struct Case {
    /// The logical discriminant bits.
    discriminant: mir::Discriminant,
    /// The logical payload type.
    ty: mir::TypeId,
    /// The payload register representation.
    representation: mir::Representation,
    /// The largest invalid scalar range in the payload.
    niche: Option<mir::ScalarField>,
    /// The payload size in bytes.
    size: u32,
    /// The payload alignment in bytes.
    alignment: u32,
    /// The payload reference trace map.
    trace_map: mir::TraceMap,
}

impl Variant {
    /// Compute the case layouts required to construct one variant.
    pub(super) fn new(
        discriminant: mir::TypeId,
        cases: &[mir::VariantCase],
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
    pub(super) fn layout(self, module: ModuleId) -> Result<mir::Layout, LayoutError> {
        if let Some(layout) = self.niche()? {
            return Ok(layout);
        }

        self.direct(module)
    }

    /// Encode this variant in one payload niche when possible.
    fn niche(&self) -> Result<Option<mir::Layout>, LayoutError> {
        if self.cases.len() < 2 {
            return Ok(None);
        }

        // require exactly one case containing physical bytes
        let mut untagged = None;
        for (index, case) in self.cases.iter().enumerate() {
            if case.size == 0 {
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
        let required = (self.cases.len() - 1) as u128;
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
        let expanded = mir::Scalar::with_validity(
            scalar.primitive,
            mir::Validity::new(scalar.validity.start.bits(), end),
        );
        let niche = if niche_count == required {
            None
        } else {
            expanded.niche(field.offset)
        };
        let representation = match payload.representation {
            mir::Representation::Scalar(_) if field.offset == 0 => {
                mir::Representation::Scalar(expanded)
            }
            mir::Representation::ScalarPair(mut fields) => {
                let Some(candidate) = fields
                    .iter_mut()
                    .find(|candidate| candidate.offset == field.offset)
                else {
                    return Err(LayoutError::InvalidNiche {
                        offset: field.offset,
                    });
                };
                candidate.scalar = expanded;

                mir::Representation::ScalarPair(fields)
            }
            mir::Representation::Memory => mir::Representation::Memory,
            _ => {
                return Err(LayoutError::InvalidNiche {
                    offset: field.offset,
                });
            }
        };

        // map every encoded case around the untagged payload case
        let cases = self.case_layouts(0);
        let encoding = mir::VariantEncoding::Niche {
            field: mir::DiscriminantField {
                offset: field.offset,
                byte_len: field.scalar.bit_width().div_ceil(8) as u8,
                bit_offset: 0,
                bit_len: field.scalar.bit_width() as u8,
            },
            untagged_case: untagged_case as u32,
            niche_start,
        };
        let trace_map = self.trace(encoding, &cases);

        Ok(Some(mir::Layout {
            shape: mir::LayoutShape::Variant(mir::VariantLayout {
                discriminant: self.discriminant,
                encoding,
                cases,
            }),
            representation,
            niche,
            size: payload.size,
            alignment: payload.alignment,
            trace_map,
        }))
    }

    /// Store this variant behind one direct discriminant field.
    fn direct(self, module: ModuleId) -> Result<mir::Layout, LayoutError> {
        let mir::Representation::Scalar(tag_scalar) = self.tag.representation else {
            return Err(LayoutError::Unsupported {
                module,
                construct: "a layout for this variant discriminant".to_string(),
            });
        };
        let tag_size = self.tag.size;
        let tag_alignment = self.tag.alignment;

        // retain the tightest wrapping validity range covering every case tag
        let mask = tag_scalar.bit_mask();
        let mut values = self
            .cases
            .iter()
            .map(|case| case.discriminant.bits() & mask)
            .collect::<Vec<_>>();
        values.sort_unstable();
        values.dedup();
        let tag_scalar = if let Some(&first) = values.first() {
            let mut validity = mir::Validity::new(first, first);
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
                    validity = mir::Validity::new(next, current);
                }
            }

            mir::Scalar::with_validity(tag_scalar.primitive, validity)
        } else {
            tag_scalar
        };

        // place the shared payload after the direct tag
        let payload_offset = tag_size.next_multiple_of(self.payload_alignment.max(1));
        let alignment = tag_alignment.max(self.payload_alignment);
        let size = (payload_offset + self.payload_size).next_multiple_of(alignment);
        let cases = self.case_layouts(payload_offset);
        let encoding = mir::VariantEncoding::Direct {
            field: mir::DiscriminantField::scalar(0, tag_size as u8),
        };

        // keep direct payloads in registers only under one shared scalar primitive
        let representation = if self.payload_size == 0 {
            mir::Representation::Scalar(tag_scalar)
        } else {
            let mut payload = None;
            for case in self.cases.iter().filter(|case| case.size != 0) {
                let mir::Representation::Scalar(scalar) = case.representation else {
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
                Some(payload) => mir::Representation::ScalarPair([
                    mir::ScalarField::new(tag_scalar, 0),
                    mir::ScalarField::new(mir::Scalar::new(payload), payload_offset),
                ]),
                None => mir::Representation::Memory,
            }
        };
        let trace_map = self.trace(encoding, &cases);

        Ok(mir::Layout {
            shape: mir::LayoutShape::Variant(mir::VariantLayout {
                discriminant: self.discriminant,
                encoding,
                cases,
            }),
            representation,
            niche: tag_scalar.niche(0),
            size,
            alignment,
            trace_map,
        })
    }

    /// Return the physical case records at one shared payload offset.
    fn case_layouts(&self, payload_offset: u32) -> Vec<mir::VariantCaseLayout> {
        self.cases
            .iter()
            .map(|case| mir::VariantCaseLayout {
                discriminant: case.discriminant,
                ty: case.ty,
                payload_offset,
            })
            .collect()
    }

    /// Build the case-selected trace map for one variant encoding.
    fn trace(
        &self,
        encoding: mir::VariantEncoding,
        cases: &[mir::VariantCaseLayout],
    ) -> mir::TraceMap {
        let cases = cases
            .iter()
            .zip(&self.cases)
            .map(|(layout, case)| mir::VariantTrace {
                discriminant: layout.discriminant,
                payload_offset: layout.payload_offset,
                map: case.trace_map.clone(),
            })
            .collect::<Vec<_>>();
        if cases.iter().all(|case| !case.map.has_reference()) {
            mir::TraceMap::Empty
        } else {
            mir::TraceMap::Variant {
                encoding,
                cases: cases.into_boxed_slice(),
            }
        }
    }
}
