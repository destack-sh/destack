use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, answer};

use super::query::LayoutQuery;
use super::scalar::{align_to, smallest_tag_bytes};

/// One aggregate field whose value occupies storage.
#[derive(Clone, Copy)]
pub(super) struct AggregateSlot {
    /// The field key, when the slot is named.
    pub(super) key: Option<dir::StaticKey>,
    /// The field type.
    pub(super) ty: dir::GlobalTypeId,
    /// The source declaration for diagnostics.
    pub(super) source: Option<dir::GlobalNodeIdAny>,
}

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
        owner: ModuleId,
        element: dir::GlobalTypeId,
        count: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        // compute the element layout and static length
        let source = self
            .check
            .origin_source_node(self.origin)?
            .into_global(self.origin.module());
        let Some(layout_id) = answer!(self.slot_layout(owner, element, source)?) else {
            return Ok(Answer::Ready(None));
        };
        let (element_size, element_alignment, element_niche) = {
            let layout = self.layout(owner, layout_id);
            (layout.size, layout.alignment, layout.niche)
        };

        let origin = self.origin;
        let count = answer!(self.check.reduce_type_head(origin, count)?);
        let length = match self.check.ty(count)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => u32::try_from(*value).ok(),
            _ => None,
        };
        let Some(length) = length else {
            return Ok(Answer::Ready(None));
        };

        let stride = align_to(element_size, element_alignment);

        Ok(Answer::Ready(Some(dir::Layout {
            shape: dir::LayoutShape::Array(dir::ElementLayout {
                element,
                stride,
                count: length,
            }),
            size: stride.saturating_mul(length),
            alignment: element_alignment,
            // keep the first element niche
            niche: (length > 0).then_some(element_niche).flatten(),
        })))
    }

    /// Compute one ordered aggregate layout.
    pub(super) fn aggregate_layout(
        &mut self,
        owner: ModuleId,
        fields: &[AggregateSlot],
        shape: AggregateLayout,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        let mut offset = 0u32;
        let mut alignment = 1u32;
        let mut layout_fields = Vec::with_capacity(fields.len());
        let mut niche: Option<dir::Niche> = None;
        let origin_source = self
            .check
            .origin_source_node(self.origin)?
            .into_global(self.origin.module());

        for field in fields.iter().copied() {
            let source = field.source.unwrap_or(origin_source);
            let Some(layout_id) = answer!(self.slot_layout(owner, field.ty, source)?) else {
                return Ok(Answer::Ready(None));
            };
            let (field_size, field_alignment, field_niche) = {
                let layout = self.layout(owner, layout_id);
                (layout.size, layout.alignment, layout.niche)
            };
            let field_offset = align_to(offset, field_alignment);

            // keep the largest niche shifted to its field offset
            if let Some(slot_niche) = field_niche {
                let shifted = dir::Niche {
                    offset: field_offset + slot_niche.offset,
                    ..slot_niche
                };
                if niche.is_none_or(|kept| shifted.free_values() > kept.free_values()) {
                    niche = Some(shifted);
                }
            }

            offset = field_offset.saturating_add(field_size);
            alignment = alignment.max(field_alignment);
            layout_fields.push(dir::LayoutField {
                key: field.key,
                ty: field.ty,
                layout: layout_id,
                offset: field_offset,
                size: field_size,
                alignment: field_alignment,
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
        owner: ModuleId,
        elements: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        // compute every variant layout first
        let mut cases = Vec::with_capacity(elements.len());
        let mut layouts =
            SmallVec::<[(dir::LocalLayoutId, u32, u32, Option<dir::Niche>); 4]>::new();
        let source = self
            .check
            .origin_source_node(self.origin)?
            .into_global(self.origin.module());
        for element in elements.iter().copied() {
            let Some(layout_id) = answer!(self.slot_layout(owner, element, source)?) else {
                return Ok(Answer::Ready(None));
            };
            let (size, alignment, niche) = {
                let layout = self.layout(owner, layout_id);
                (layout.size, layout.alignment, layout.niche)
            };

            cases.push(dir::VariantCaseLayout {
                ty: element,
                layout: layout_id,
            });
            layouts.push((layout_id, size, alignment, niche));
        }

        // pack unit variants into one niched payload when it fits
        let unit_count = layouts.iter().filter(|(_, size, _, _)| *size == 0).count() as u128;
        let payload_count = layouts.len() as u128 - unit_count;
        if payload_count == 1 {
            let (_, payload_size, payload_alignment, payload_niche) = layouts
                .iter()
                .find(|(_, size, _, _)| *size != 0)
                .unwrap_or_else(|| unreachable!("union payload variant must exist"));

            if let Some(niche) = payload_niche
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
                    size: *payload_size,
                    alignment: *payload_alignment,
                    // spend the niche encoding unit variants
                    niche: None,
                })));
            }
        }

        // tag with the smallest unsigned integer that fits every case
        let tag_size = smallest_tag_bytes(layouts.len());
        let mut payload_size = 0u32;
        let mut payload_alignment = 1u32;
        for (_, size, alignment, _) in &layouts {
            payload_size = payload_size.max(*size);
            payload_alignment = payload_alignment.max(*alignment);
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
                end: layouts.len().saturating_sub(1) as u128,
            }),
        })))
    }
}
