use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, answer};

use super::memory::SlotLayout;
use super::query::LayoutQuery;
use super::scalar::{align_to, smallest_tag_bytes};

/// Aggregate layout shape selected by source type.
pub(super) enum AggregateLayout {
    /// Struct storage.
    Struct,
    /// Object storage with a dispatch table header.
    Object,
    /// Tuple storage.
    Tuple,
}

impl LayoutQuery<'_, '_> {
    /// Compute one fixed array layout.
    pub(super) fn fixed_array_layout(
        &mut self,
        element: dir::GlobalTypeId,
        count: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        // close the element layout and the static length
        let Some(slot) = answer!(self.slot_layout(element)?) else {
            return Ok(Answer::Ready(None));
        };
        let origin = self.origin;
        let count = answer!(self.check.reduce_type_root(origin, count)?);
        let length = match self.check.ty(count)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => u32::try_from(*value).ok(),
            _ => None,
        };
        let Some(length) = length else {
            return Ok(Answer::Ready(None));
        };

        let stride = align_to(slot.size, slot.alignment);

        Ok(Answer::Ready(Some(dir::Layout {
            shape: dir::LayoutShape::Array(dir::ElementLayout {
                element,
                stride,
                count: length,
            }),
            size: stride.saturating_mul(length),
            alignment: slot.alignment,
            // keep the first element niche
            niche: (length > 0).then_some(slot.niche).flatten(),
        })))
    }

    /// Compute one ordered aggregate layout.
    /// TODO #Incomplete: apply representation decorators to field and aggregate
    /// alignment once decorator capture wires them through.
    pub(super) fn aggregate_layout(
        &mut self,
        fields: &[(Option<dir::StaticKey>, dir::GlobalTypeId)],
        shape: AggregateLayout,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        let mut offset = 0u32;
        let mut alignment = 1u32;
        let mut layout_fields = Vec::with_capacity(fields.len());
        let mut niche: Option<dir::Niche> = None;

        for (key, ty) in fields.iter().copied() {
            let Some(slot) = answer!(self.slot_layout(ty)?) else {
                return Ok(Answer::Ready(None));
            };
            let field_offset = align_to(offset, slot.alignment);

            // keep the largest niche shifted to its field offset
            if let Some(slot_niche) = slot.niche {
                let shifted = dir::Niche {
                    offset: field_offset + slot_niche.offset,
                    ..slot_niche
                };
                if niche.is_none_or(|kept| shifted.free_values() > kept.free_values()) {
                    niche = Some(shifted);
                }
            }

            offset = field_offset.saturating_add(slot.size);
            alignment = alignment.max(slot.alignment);
            layout_fields.push(dir::LayoutField {
                key,
                ty,
                layout: slot.id,
                offset: field_offset,
                size: slot.size,
                alignment: slot.alignment,
            });
        }

        let shape = match shape {
            AggregateLayout::Struct => dir::LayoutShape::Struct(dir::StructLayout {
                fields: layout_fields,
            }),
            AggregateLayout::Object => dir::LayoutShape::Object(dir::ObjectLayout {
                fields: layout_fields,
            }),
            AggregateLayout::Tuple => dir::LayoutShape::Tuple(dir::TupleLayout {
                elements: layout_fields,
            }),
        };

        Ok(Answer::Ready(Some(dir::Layout {
            shape,
            size: align_to(offset, alignment),
            alignment,
            niche,
        })))
    }

    /// Compute one union layout, hunting niches before adding a tag.
    pub(super) fn union_layout(
        &mut self,
        elements: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        // close every variant layout first
        let mut cases = Vec::with_capacity(elements.len());
        let mut slots = SmallVec::<[SlotLayout; 4]>::new();
        for element in elements.iter().copied() {
            let Some(slot) = answer!(self.slot_layout(element)?) else {
                return Ok(Answer::Ready(None));
            };

            cases.push(dir::VariantCaseLayout {
                ty: element,
                layout: slot.id,
            });
            slots.push(slot);
        }

        // pack unit variants into one niched payload when it fits
        let unit_count = slots.iter().filter(|slot| slot.size == 0).count() as u128;
        let payload_count = slots.len() as u128 - unit_count;
        if payload_count == 1 {
            let payload = slots
                .iter()
                .find(|slot| slot.size != 0)
                .copied()
                .unwrap_or_else(|| unreachable!("union payload variant must exist"));

            if let Some(niche) = payload.niche
                && niche.free_values() >= unit_count
            {
                return Ok(Answer::Ready(Some(dir::Layout {
                    shape: dir::LayoutShape::Variant(dir::VariantLayout {
                        tag: dir::VariantTagLayout {
                            ty: None,
                            size: 0,
                            alignment: 1,
                        },
                        payload_offset: Some(0),
                        variants: cases,
                    }),
                    size: payload.size,
                    alignment: payload.alignment,
                    // spend the niche encoding unit variants
                    niche: None,
                })));
            }
        }

        // tag with the smallest unsigned integer that fits every case
        let tag_size = smallest_tag_bytes(slots.len());
        let mut payload_size = 0u32;
        let mut payload_alignment = 1u32;
        for slot in &slots {
            payload_size = payload_size.max(slot.size);
            payload_alignment = payload_alignment.max(slot.alignment);
        }
        let payload_offset = align_to(tag_size, payload_alignment);
        let alignment = tag_size.max(payload_alignment);
        let size = align_to(payload_offset.saturating_add(payload_size), alignment);

        Ok(Answer::Ready(Some(dir::Layout {
            shape: dir::LayoutShape::Variant(dir::VariantLayout {
                tag: dir::VariantTagLayout {
                    ty: None,
                    size: tag_size,
                    alignment: tag_size,
                },
                payload_offset: Some(payload_offset),
                variants: cases,
            }),
            size,
            alignment,
            // reserve tag values above the case count as a niche
            niche: Some(dir::Niche {
                offset: 0,
                width: tag_size,
                start: 0,
                end: slots.len().saturating_sub(1) as u128,
            }),
        })))
    }
}
