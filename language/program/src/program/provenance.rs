use destack_core::{Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_serde::Reflect;
use destack_source::{Location, LocationId, ProvenanceId, Span, Transform, TransformId};
use serde::{Deserialize, Serialize};

/// Compilation provenance stored in one Program image.
#[repr(C)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ProvenanceTable {
    /// Locations in allocation order.
    locations: SectionSlice<Location>,
    /// Packed locations referenced by fused locations.
    fused_locations: SectionSlice<LocationId>,
    /// Direct attribution indexed by provenance id.
    attributions: SectionSlice<LocationId>,
    /// Producing transforms indexed by provenance id.
    producers: SectionSlice<Optional<TransformId>>,
    /// Transforms in execution order.
    transforms: SectionSlice<Transform>,
    /// Packed transform inputs.
    inputs: SectionSlice<ProvenanceId>,
    /// Packed transform outputs.
    outputs: SectionSlice<ProvenanceId>,
    /// Consumer offsets by provenance index.
    consumer_offsets: SectionSlice<u32>,
    /// Consumer transforms grouped by provenance id.
    consumers: SectionSlice<TransformId>,
}

impl ProvenanceTable {
    /// Pack one segmented provenance table into Program sections.
    pub(crate) fn pack(
        table: &destack_source::ProvenanceTable,
        sections: &mut SectionBuilder,
    ) -> Self {
        let mut locations = Vec::new();
        let mut fused_locations = Vec::new();
        let mut attributions = Vec::new();
        let mut producers = Vec::new();
        let mut transforms = Vec::new();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        // flatten immutable segments and rebase their packed ranges
        for segment in table.segments() {
            let fused_location_base = fused_locations.len() as u32;
            let input_base = inputs.len() as u32;
            let output_base = outputs.len() as u32;

            locations.extend(segment.locations().iter().copied().map(|location| {
                if let Location::Fused(mut range) = location {
                    range.start += fused_location_base;

                    Location::Fused(range)
                } else {
                    location
                }
            }));
            transforms.extend(segment.transforms().iter().copied().map(|mut transform| {
                transform.inputs.start += input_base;
                transform.outputs.start += output_base;

                transform
            }));
            fused_locations.extend_from_slice(segment.fused_locations());
            attributions.extend_from_slice(segment.attribution());
            producers.extend_from_slice(segment.producers());
            inputs.extend_from_slice(segment.inputs());
            outputs.extend_from_slice(segment.outputs());
        }

        // materialize the reverse graph index for interactive queries
        let (consumer_offsets, consumers) =
            Self::index_consumers(&attributions, &transforms, &inputs);

        Self {
            locations: sections.insert(locations),
            fused_locations: sections.insert(fused_locations),
            attributions: sections.insert(attributions),
            producers: sections.insert(producers),
            transforms: sections.insert(transforms),
            inputs: sections.insert(inputs),
            outputs: sections.insert(outputs),
            consumer_offsets: sections.insert(consumer_offsets),
            consumers: sections.insert(consumers),
        }
    }

    /// Return whether every graph reference and packed range is valid.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let locations = self.locations(sections);
        let fused_locations = self.fused_locations(sections);
        let attributions = self.attributions(sections);
        let producers = self.producers(sections);
        let transforms = self.transforms(sections);
        let inputs = self.input_entries(sections);
        let outputs = self.output_entries(sections);

        // validate location references and direct attribution
        if attributions.len() != producers.len()
            || !Self::locations_fit(locations, fused_locations)
            || attributions
                .iter()
                .any(|location| location.0 as usize >= locations.len())
        {
            return false;
        }

        // rebuild both derived graph columns from the transform lists
        let Some((expected_producers, expected_offsets, expected_consumers)) =
            Self::graph_columns(attributions.len(), transforms, inputs, outputs)
        else {
            return false;
        };
        if producers != expected_producers
            || self.consumer_offsets(sections) != expected_offsets
            || self.consumer_entries(sections) != expected_consumers
        {
            return false;
        }

        // require every root provenance to refer directly to authored text
        attributions
            .iter()
            .copied()
            .zip(producers)
            .all(|(location, producer)| {
                producer.get().is_some()
                    || matches!(locations[location.0 as usize], Location::Authored { .. })
            })
    }

    /// Return locations in allocation order.
    pub fn locations<'a>(&self, sections: SectionImage<'a>) -> &'a [Location] {
        sections.entries(self.locations)
    }

    /// Return packed locations referenced by fused locations.
    pub fn fused_locations<'a>(&self, sections: SectionImage<'a>) -> &'a [LocationId] {
        sections.entries(self.fused_locations)
    }

    /// Return direct attribution indexed by provenance id.
    pub fn attributions<'a>(&self, sections: SectionImage<'a>) -> &'a [LocationId] {
        sections.entries(self.attributions)
    }

    /// Return producing transforms indexed by provenance id.
    pub fn producers<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<TransformId>] {
        sections.entries(self.producers)
    }

    /// Return transforms in execution order.
    pub fn transforms<'a>(&self, sections: SectionImage<'a>) -> &'a [Transform] {
        sections.entries(self.transforms)
    }

    /// Return whether this table contains every given provenance id.
    pub(super) fn contains_all(
        &self,
        sections: SectionImage<'_>,
        ids: impl IntoIterator<Item = ProvenanceId>,
    ) -> bool {
        let count = self.attributions(sections).len();

        ids.into_iter().all(|id| (id.index() as usize) < count)
    }

    /// Return the locations represented by one fused location.
    pub fn fused<'a>(
        &self,
        sections: SectionImage<'a>,
        id: LocationId,
    ) -> Option<&'a [LocationId]> {
        let Location::Fused(locations) = self.location(sections, id)? else {
            return None;
        };
        let start = locations.start as usize;
        let end = start + locations.len as usize;

        self.fused_locations(sections).get(start..end)
    }

    /// Return the causal inputs of one compiler transform.
    pub fn inputs<'a>(
        &self,
        sections: SectionImage<'a>,
        id: TransformId,
    ) -> Option<&'a [ProvenanceId]> {
        let transform = self.transform(sections, id)?;
        let start = transform.inputs.start as usize;
        let end = start + transform.inputs.len as usize;

        self.input_entries(sections).get(start..end)
    }

    /// Return the outputs produced by one compiler transform.
    pub fn outputs<'a>(
        &self,
        sections: SectionImage<'a>,
        id: TransformId,
    ) -> Option<&'a [ProvenanceId]> {
        let transform = self.transform(sections, id)?;
        let start = transform.outputs.start as usize;
        let end = start + transform.outputs.len as usize;

        self.output_entries(sections).get(start..end)
    }

    /// Return the transforms that consume one provenance id.
    pub fn consumers<'a>(
        &self,
        sections: SectionImage<'a>,
        id: ProvenanceId,
    ) -> Option<&'a [TransformId]> {
        let index = id.index() as usize;
        let offsets = self.consumer_offsets(sections);
        let start = *offsets.get(index)? as usize;
        let end = *offsets.get(index + 1)? as usize;

        self.consumer_entries(sections).get(start..end)
    }

    /// Return the roots and every provenance id that causally precedes them.
    pub fn ancestors(
        &self,
        sections: SectionImage<'_>,
        roots: &[ProvenanceId],
    ) -> Option<Vec<ProvenanceId>> {
        let mut selected = vec![false; self.attributions(sections).len()];
        let mut pending = roots.to_vec();

        // traverse producer inputs once per provenance id
        while let Some(id) = pending.pop() {
            let visited = selected.get_mut(id.index() as usize)?;
            if *visited {
                continue;
            }
            *visited = true;

            if let Some(producer) = self.producer(sections, id) {
                pending.extend(self.inputs(sections, producer)?.iter().copied());
            }
        }

        Some(Self::selected_provenance(selected))
    }

    /// Return the roots and every provenance id that causally follows them.
    pub fn descendants(
        &self,
        sections: SectionImage<'_>,
        roots: &[ProvenanceId],
    ) -> Option<Vec<ProvenanceId>> {
        let mut selected = vec![false; self.attributions(sections).len()];
        let mut pending = roots.to_vec();

        // traverse consumer outputs once per provenance id
        while let Some(id) = pending.pop() {
            let visited = selected.get_mut(id.index() as usize)?;
            if *visited {
                continue;
            }
            *visited = true;

            for consumer in self.consumers(sections, id)? {
                pending.extend(self.outputs(sections, *consumer)?.iter().copied());
            }
        }

        Some(Self::selected_provenance(selected))
    }

    /// Return the direct attribution of one provenance id.
    pub fn attribution(&self, sections: SectionImage<'_>, id: ProvenanceId) -> Option<LocationId> {
        self.attributions(sections)
            .get(id.index() as usize)
            .copied()
    }

    /// Return the transform that produced one provenance id.
    pub fn producer(&self, sections: SectionImage<'_>, id: ProvenanceId) -> Option<TransformId> {
        self.producers(sections).get(id.index() as usize)?.get()
    }

    /// Return one location.
    pub fn location(&self, sections: SectionImage<'_>, id: LocationId) -> Option<Location> {
        self.locations(sections).get(id.0 as usize).copied()
    }

    /// Return one compiler transform.
    pub fn transform(&self, sections: SectionImage<'_>, id: TransformId) -> Option<Transform> {
        self.transforms(sections).get(id.0 as usize).copied()
    }

    /// Return the complete authored span represented by one provenance id.
    pub fn span(&self, sections: SectionImage<'_>, id: ProvenanceId) -> Option<Span> {
        let location = self.attribution(sections, id)?;

        self.location_span(sections, location, |span, _| Some(span))
    }

    /// Return the primary authored span represented by one provenance id.
    pub fn primary_span(&self, sections: SectionImage<'_>, id: ProvenanceId) -> Option<Span> {
        let location = self.attribution(sections, id)?;

        self.location_span(sections, location, |_, primary| primary.get())
    }

    /// Return packed transform inputs.
    fn input_entries<'a>(&self, sections: SectionImage<'a>) -> &'a [ProvenanceId] {
        sections.entries(self.inputs)
    }

    /// Return packed transform outputs.
    fn output_entries<'a>(&self, sections: SectionImage<'a>) -> &'a [ProvenanceId] {
        sections.entries(self.outputs)
    }

    /// Return consumer offsets by provenance index.
    fn consumer_offsets<'a>(&self, sections: SectionImage<'a>) -> &'a [u32] {
        sections.entries(self.consumer_offsets)
    }

    /// Return consumer transforms grouped by provenance id.
    fn consumer_entries<'a>(&self, sections: SectionImage<'a>) -> &'a [TransformId] {
        sections.entries(self.consumers)
    }

    /// Return one span shared by every represented authored location.
    fn location_span(
        &self,
        sections: SectionImage<'_>,
        id: LocationId,
        authored: impl Fn(Span, Optional<Span>) -> Option<Span>,
    ) -> Option<Span> {
        let mut pending = vec![id];
        let mut common = None;

        // project nested expansions and fused locations without recursion
        while let Some(id) = pending.pop() {
            match self.location(sections, id)? {
                Location::Authored { span, primary } => {
                    let span = authored(span, primary)?;
                    if common.is_some_and(|common| common != span) {
                        return None;
                    }
                    common = Some(span);
                }
                Location::Expanded { source, .. } => pending.push(source),
                Location::Fused(_) => {
                    pending.extend(self.fused(sections, id)?.iter().rev().copied())
                }
                Location::Generated => return None,
            }
        }

        common
    }

    /// Return whether every location references a valid preceding location.
    fn locations_fit(locations: &[Location], fused_locations: &[LocationId]) -> bool {
        locations.iter().enumerate().all(|(index, location)| {
            let precedes = |id: LocationId| id.0 < index as u32;

            match *location {
                Location::Authored { span, primary } => {
                    span.start <= span.end
                        && primary
                            .get()
                            .is_none_or(|primary| span.contains_span(primary))
                }
                Location::Expanded { source, site } => precedes(source) && precedes(site),
                Location::Fused(range) => {
                    !range.is_empty()
                        && range.fits(fused_locations.len())
                        && range.slice(fused_locations).iter().copied().all(precedes)
                }
                Location::Generated => true,
            }
        })
    }

    /// Rebuild the producer and consumer columns from packed transforms.
    fn graph_columns(
        provenance_count: usize,
        transforms: &[Transform],
        inputs: &[ProvenanceId],
        outputs: &[ProvenanceId],
    ) -> Option<(Vec<Optional<TransformId>>, Vec<u32>, Vec<TransformId>)> {
        let mut producers = vec![Optional::none(); provenance_count];
        let mut consumer_counts = vec![0u32; provenance_count];

        // assign each output once and count each input use
        for (index, transform) in transforms.iter().enumerate() {
            if !transform.inputs.fits(inputs.len())
                || !transform.outputs.fits(outputs.len())
                || transform.inputs.is_empty() && transform.outputs.is_empty()
            {
                return None;
            }
            let id = TransformId(index as u32);
            let transform_inputs = transform.inputs.slice(inputs);
            let transform_outputs = transform.outputs.slice(outputs);

            for input in transform_inputs {
                let count = consumer_counts.get_mut(input.index() as usize)?;
                *count = count.checked_add(1)?;
            }
            for output in transform_outputs {
                if transform_inputs
                    .iter()
                    .any(|input| input.index() >= output.index())
                {
                    return None;
                }
                let producer = producers.get_mut(output.index() as usize)?;
                if producer.get().is_some() {
                    return None;
                }
                *producer = Optional::some(id);
            }
        }

        // prefix the consumer counts into one compact range column
        let mut consumer_offsets = Vec::with_capacity(provenance_count + 1);
        consumer_offsets.push(0u32);
        for count in consumer_counts {
            let offset = consumer_offsets.last()?.checked_add(count)?;
            consumer_offsets.push(offset);
        }
        if consumer_offsets.last().copied()? as usize != inputs.len() {
            return None;
        }

        // group consumer transforms by their input provenance
        let mut cursors = consumer_offsets[..provenance_count].to_vec();
        let mut consumers = vec![TransformId(0); inputs.len()];
        for (index, transform) in transforms.iter().enumerate() {
            let id = TransformId(index as u32);
            for input in transform.inputs.slice(inputs) {
                let cursor = cursors.get_mut(input.index() as usize)?;
                *consumers.get_mut(*cursor as usize)? = id;
                *cursor += 1;
            }
        }

        Some((producers, consumer_offsets, consumers))
    }

    /// Index consumer transforms for one trusted source provenance table.
    fn index_consumers(
        attributions: &[LocationId],
        transforms: &[Transform],
        inputs: &[ProvenanceId],
    ) -> (Vec<u32>, Vec<TransformId>) {
        let mut counts = vec![0u32; attributions.len()];
        for input in inputs {
            counts[input.index() as usize] += 1;
        }

        let mut offsets = Vec::with_capacity(attributions.len() + 1);
        let mut offset = 0u32;
        offsets.push(offset);
        for count in counts {
            offset += count;
            offsets.push(offset);
        }

        let mut cursors = offsets[..attributions.len()].to_vec();
        let mut consumers = vec![TransformId(0); inputs.len()];
        for (index, transform) in transforms.iter().enumerate() {
            let id = TransformId(index as u32);
            for input in transform.inputs.slice(inputs) {
                let cursor = &mut cursors[input.index() as usize];
                consumers[*cursor as usize] = id;
                *cursor += 1;
            }
        }

        (offsets, consumers)
    }

    /// Collect a dense provenance selection in graph order.
    fn selected_provenance(selected: Vec<bool>) -> Vec<ProvenanceId> {
        selected
            .into_iter()
            .enumerate()
            .filter_map(|(index, is_selected)| {
                is_selected.then_some(ProvenanceId::new(index as u32))
            })
            .collect()
    }
}
