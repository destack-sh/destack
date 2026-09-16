use std::cmp::Reverse;

use destack_core::StringId;

use crate::{LayoutField, Representation, ScalarField, TraceMap, TypeId};

use super::{LayoutBuilder, LayoutError};

/// One completely packed aggregate layout.
pub(super) struct Aggregate {
    /// The fields in source order with their physical offsets.
    pub(super) fields: Vec<LayoutField>,
    /// The aggregate register representation.
    pub(super) representation: Representation,
    /// The largest invalid scalar range in the aggregate.
    pub(super) niche: Option<ScalarField>,
    /// The aggregate size in bytes.
    pub(super) size: u32,
    /// The aggregate alignment in bytes.
    pub(super) alignment: u32,
    /// The aggregate reference trace map.
    pub(super) trace_map: TraceMap,
    /// Whether one field admits no value.
    pub(super) uninhabited: bool,
}

impl Aggregate {
    /// Pack one aggregate from fields in source order.
    pub(super) fn new(
        components: &[(Option<StringId>, TypeId)],
        layouts: &mut LayoutBuilder<'_>,
    ) -> Result<Self, LayoutError> {
        // compute each field layout before physical ordering
        let mut computed = Vec::with_capacity(components.len());
        for (source_index, (name, ty)) in components.iter().enumerate() {
            let layout = layouts.layout_type(*ty)?;
            computed.push((source_index, *name, *ty, layout));
        }

        // place fields by alignment, retaining source order as the tiebreak
        computed.sort_by_key(|(index, _, _, layout)| {
            (Reverse(layouts.layout(*layout).alignment), *index)
        });
        let mut offset = 0u32;
        let mut alignment = 1u32;
        let mut placed = Vec::with_capacity(computed.len());
        let mut traces = Vec::with_capacity(computed.len());
        for (source_index, name, ty, layout_id) in computed {
            let layout = layouts.layout(layout_id);
            offset = offset.next_multiple_of(layout.alignment.max(1));
            alignment = alignment.max(layout.alignment);
            traces.push(TraceMap::nested(offset, layout.trace_map.clone()));
            let field = LayoutField {
                name,
                ty,
                offset,
                size: layout.size,
                alignment: layout.alignment,
                source_index: source_index as u32,
            };
            offset += layout.size;
            placed.push((field, layout_id));
        }

        // derive the register representation and largest nested niche
        placed.sort_by_key(|(field, _)| field.source_index);
        let mut scalars = Vec::with_capacity(2);
        let mut is_register = true;
        let mut niche = None;
        let mut uninhabited = false;
        for (field, layout_id) in &placed {
            let layout = layouts.layout(*layout_id);
            uninhabited |= layout.uninhabited;
            if field.size != 0 {
                match layout.representation {
                    Representation::Scalar(scalar) => {
                        scalars.push(ScalarField::new(scalar, field.offset));
                    }
                    _ => is_register = false,
                }
            }

            if let Some(mut candidate) = layout.niche {
                candidate.offset += field.offset;
                if niche.is_none_or(|current: ScalarField| {
                    candidate.invalid_count() > current.invalid_count()
                }) {
                    niche = Some(candidate);
                }
            }
        }
        let representation = if !is_register {
            Representation::Memory
        } else {
            match scalars.as_slice() {
                [field] if field.offset == 0 => Representation::Scalar(field.scalar),
                [first, second] => {
                    let mut fields = [*first, *second];
                    fields.sort_by_key(|field| field.offset);

                    Representation::ScalarPair(fields)
                }
                _ => Representation::Memory,
            }
        };

        // seal the complete aggregate layout
        let fields = placed.into_iter().map(|(field, _)| field).collect();
        let size = offset.next_multiple_of(alignment);
        let trace_map = TraceMap::composite(traces);

        Ok(Self {
            fields,
            representation,
            niche,
            size,
            alignment,
            trace_map,
            uninhabited,
        })
    }
}
