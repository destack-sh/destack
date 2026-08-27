use std::sync::Arc;

use destack_core::{EntryRange, Optional, StringId, StringPool};

use crate::Span;

use super::{
    Location, LocationId, ProvenanceId, ProvenanceJournal, ProvenanceSegment, ProvenanceTable,
    Transform, TransformId,
};

/// One mutable append batch for a provenance table.
#[derive(Debug)]
pub struct ProvenanceBuilder {
    /// Stable transform names.
    names: StringPool,
    /// Finished append batches.
    segments: Vec<Arc<ProvenanceSegment>>,
    /// The first provenance id allocated by the open batch.
    first_provenance: ProvenanceId,
    /// The first location allocated by the open batch.
    first_location: LocationId,
    /// The first transform allocated by the open batch.
    first_transform: TransformId,
    /// Locations allocated by the open batch.
    locations: Vec<Location>,
    /// Locations referenced by fused locations in the open batch.
    fused_locations: Vec<LocationId>,
    /// Direct attribution indexed by provenance id.
    attribution: Vec<LocationId>,
    /// Producing transforms indexed by provenance id.
    producers: Vec<Optional<TransformId>>,
    /// Transforms allocated by the open batch.
    transforms: Vec<Transform>,
    /// Transform inputs allocated by the open batch.
    inputs: Vec<ProvenanceId>,
    /// Transform outputs allocated by the open batch.
    outputs: Vec<ProvenanceId>,
}

/// One restorable position in an open provenance append batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProvenanceMark {
    /// The number of locations.
    locations: usize,
    /// The number of fused locations.
    fused_locations: usize,
    /// The number of provenance ids.
    provenance: usize,
    /// The number of compiler transforms.
    transforms: usize,
    /// The number of transform inputs.
    inputs: usize,
    /// The number of transform outputs.
    outputs: usize,
}

/// A provenance id mapping produced by one table import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceRemap {
    /// The number of provenance ids in the source table.
    source_count: u32,
    /// The number of shared provenance ids that remain unchanged.
    shared_count: u32,
    /// The first destination index of the copied source suffix.
    copied_start: u32,
}

impl ProvenanceRemap {
    /// Create one remap for a shared prefix and contiguous copied suffix.
    fn new(source_count: u32, shared_count: u32, copied_start: u32) -> Self {
        Self {
            source_count,
            shared_count,
            copied_start,
        }
    }

    /// Create one identity remap.
    fn identity(source_count: u32) -> Self {
        Self::new(source_count, source_count, source_count)
    }

    /// Return the destination of one source provenance id.
    pub fn get(&self, id: ProvenanceId) -> Option<ProvenanceId> {
        let source = id.index();
        if source >= self.source_count {
            return None;
        }
        let destination = if source < self.shared_count {
            source
        } else {
            self.copied_start + source - self.shared_count
        };

        Some(ProvenanceId::new(destination))
    }

    /// Return the destination of one source provenance id.
    fn destination(&self, id: ProvenanceId) -> ProvenanceId {
        self.get(id)
            .unwrap_or_else(|| panic!("provenance {} is outside the source table", id.index()))
    }
}

impl ProvenanceBuilder {
    /// Create a builder for an empty provenance table.
    pub(super) fn new() -> Self {
        Self::from(&ProvenanceTable::new())
    }

    /// Return the number of provenance ids, including the open batch.
    fn len(&self) -> usize {
        self.first_provenance.index() as usize + self.attribution.len()
    }

    /// Mark the current append position for a possible restore.
    pub fn mark(&self) -> ProvenanceMark {
        ProvenanceMark {
            locations: self.locations.len(),
            fused_locations: self.fused_locations.len(),
            provenance: self.attribution.len(),
            transforms: self.transforms.len(),
            inputs: self.inputs.len(),
            outputs: self.outputs.len(),
        }
    }

    /// Restore the open append batch to one prior mark.
    pub fn restore(&mut self, mark: ProvenanceMark) {
        self.locations.truncate(mark.locations);
        self.fused_locations.truncate(mark.fused_locations);
        self.attribution.truncate(mark.provenance);
        self.producers.truncate(mark.provenance);
        self.transforms.truncate(mark.transforms);
        self.inputs.truncate(mark.inputs);
        self.outputs.truncate(mark.outputs);
    }

    /// Insert provenance for one authored construct.
    pub fn insert_authored(&mut self, span: Span) -> ProvenanceId {
        let location = self.insert_location(Location::Authored {
            span,
            primary: Optional::none(),
        });
        let id = self.next_provenance();
        self.attribution.push(location);
        self.producers.push(Optional::none());

        id
    }

    /// Set the final spans of one open authored provenance id.
    pub fn set_authored(&mut self, id: ProvenanceId, span: Span, primary: Option<Span>) {
        let index = id
            .index()
            .checked_sub(self.first_provenance.index())
            .map(|index| index as usize)
            .unwrap_or_else(|| panic!("provenance {} is outside the open batch", id.index()));
        let attribution = self.attribution[index];
        let location_index = attribution
            .0
            .checked_sub(self.first_location.0)
            .map(|index| index as usize)
            .unwrap_or_else(|| panic!("location {} is outside the open batch", attribution.0));
        let location = &mut self.locations[location_index];
        debug_assert!(
            self.producers[index].get().is_none()
                && matches!(location, Location::Authored { .. })
                && primary.is_none_or(|primary| span.contains_span(primary))
        );

        *location = Location::Authored {
            span,
            primary: Optional::from(primary),
        };
    }

    /// Insert one location.
    fn insert_location(&mut self, location: Location) -> LocationId {
        let id = self.next_location();
        self.locations.push(location);

        id
    }

    /// Insert one compiler generated location.
    pub(super) fn insert_generated(&mut self) -> LocationId {
        self.insert_location(Location::Generated)
    }

    /// Insert one expanded location from its source and expansion site.
    pub(super) fn insert_expanded(
        &mut self,
        source: ProvenanceId,
        site: ProvenanceId,
    ) -> LocationId {
        let source = self.location_of(source);
        let site = self.location_of(site);

        self.insert_location(Location::Expanded { source, site })
    }

    /// Insert one fused location containing the given provenance.
    pub(super) fn insert_fused(&mut self, provenance: &[ProvenanceId]) -> LocationId {
        assert!(
            !provenance.is_empty(),
            "a fused location requires input provenance"
        );
        let start = self.fused_locations.len() as u32;
        let len = provenance.len() as u32;
        let locations = provenance
            .iter()
            .copied()
            .map(|id| self.location_of(id))
            .collect::<Vec<_>>();
        self.fused_locations.extend(locations);

        self.insert_location(Location::Fused(EntryRange::new(start, len)))
    }

    /// Return the direct location of one provenance id.
    pub(super) fn location_of(&self, id: ProvenanceId) -> LocationId {
        if id >= self.first_provenance {
            let index = (id.index() - self.first_provenance.index()) as usize;

            self.attribution[index]
        } else {
            let Some(segment) = ProvenanceTable::provenance_segment_in(&self.segments, id) else {
                panic!("provenance {} is outside this table", id.index());
            };
            let index = (id.index() - segment.first_provenance.index()) as usize;

            segment.attribution[index]
        }
    }

    /// Record transforms bearing one stable name.
    pub fn record(&mut self, name: &str) -> ProvenanceJournal<'_> {
        let name = self.names.intern(name);

        ProvenanceJournal::new(self, name)
    }

    /// Record one compiler transform and allocate its output provenance.
    pub(super) fn transform<const OUTPUTS: usize>(
        &mut self,
        name: StringId,
        inputs: &[ProvenanceId],
        locations: [LocationId; OUTPUTS],
    ) -> [ProvenanceId; OUTPUTS] {
        let outputs = self.transform_many(name, inputs, locations);
        let Ok(outputs) = outputs.try_into() else {
            unreachable!("fixed provenance output count changed during allocation");
        };

        outputs
    }

    /// Record one compiler transform with a dynamic number of outputs.
    pub(super) fn transform_many(
        &mut self,
        name: StringId,
        inputs: &[ProvenanceId],
        locations: impl IntoIterator<Item = LocationId>,
    ) -> Vec<ProvenanceId> {
        let transform = self.next_transform();
        let input_start = u32::try_from(self.inputs.len())
            .unwrap_or_else(|_| panic!("provenance transform input table exhausted its id space"));
        let output_start = u32::try_from(self.outputs.len())
            .unwrap_or_else(|_| panic!("provenance transform output table exhausted its id space"));
        let input_count = u32::try_from(inputs.len())
            .unwrap_or_else(|_| panic!("provenance transform has too many inputs"));
        self.inputs.extend_from_slice(inputs);

        // allocate output provenance before recording the packed ids
        let outputs = locations
            .into_iter()
            .map(|location| {
                let id = self.next_provenance();
                self.attribution.push(location);
                self.producers.push(Optional::some(transform));

                id
            })
            .collect::<Vec<_>>();
        let output_count = u32::try_from(outputs.len())
            .unwrap_or_else(|_| panic!("provenance transform has too many outputs"));
        assert!(
            input_count != 0 || output_count != 0,
            "a provenance transform must consume or produce provenance"
        );
        self.outputs.extend_from_slice(&outputs);
        self.transforms.push(Transform {
            name,
            inputs: EntryRange::new(input_start, input_count),
            outputs: EntryRange::new(output_start, output_count),
        });

        outputs
    }

    /// Import one complete provenance table and return its destination ids.
    pub fn import(&mut self, table: &ProvenanceTable) -> ProvenanceRemap {
        // adopt the first imported table without copying its append batches
        if self.segments.is_empty()
            && self.locations.is_empty()
            && self.attribution.is_empty()
            && self.transforms.is_empty()
        {
            self.names = table.names.clone();
            self.segments = table.segments.clone();
            self.first_provenance = ProvenanceId::new(table.len() as u32);
            self.first_location = LocationId(table.location_count() as u32);
            self.first_transform = TransformId(table.transform_count() as u32);

            return ProvenanceRemap::identity(table.len() as u32);
        }

        self.names.ensure_all_from(&table.names);

        // share the common immutable prefix
        let common_segments = self
            .segments
            .iter()
            .zip(&table.segments)
            .take_while(|(left, right)| Arc::ptr_eq(left, right))
            .count();
        let shared = table.segments[..common_segments]
            .last()
            .map(|segment| segment.first_provenance.index() as usize + segment.attribution.len())
            .unwrap_or(0);

        // map the shared prefix and contiguous copied suffix
        let first_copied = self.len() as u32;
        let remap = ProvenanceRemap::new(table.len() as u32, shared as u32, first_copied);

        let shared_location_count = table.segments[..common_segments]
            .last()
            .map(|segment| segment.first_location.0 as usize + segment.locations.len())
            .unwrap_or(0);
        let shared_transform_count = table.segments[..common_segments]
            .last()
            .map(|segment| segment.first_transform.0 as usize + segment.transforms.len())
            .unwrap_or(0);
        let first_copied_location = self.next_location().0;
        let first_copied_transform = self.next_transform().0;
        let map_location = |id: LocationId| {
            if id.0 < shared_location_count as u32 {
                id
            } else {
                LocationId(first_copied_location + id.0 - shared_location_count as u32)
            }
        };
        let map_transform = |id: TransformId| {
            if id.0 < shared_transform_count as u32 {
                id
            } else {
                TransformId(first_copied_transform + id.0 - shared_transform_count as u32)
            }
        };

        // copy each unique location and its fused location list
        for segment in &table.segments[common_segments..] {
            for location in &segment.locations {
                let location = match *location {
                    Location::Authored { span, primary } => Location::Authored { span, primary },
                    Location::Expanded { source, site } => Location::Expanded {
                        source: map_location(source),
                        site: map_location(site),
                    },
                    Location::Fused(list) => {
                        let start = list.start as usize;
                        let end = start + list.len as usize;
                        let first = self.fused_locations.len() as u32;
                        self.fused_locations.extend(
                            segment.fused_locations[start..end]
                                .iter()
                                .copied()
                                .map(map_location),
                        );
                        Location::Fused(EntryRange::new(first, list.len))
                    }
                    Location::Generated => Location::Generated,
                };
                self.locations.push(location);
            }
        }

        // copy attribution and producers with remapped ids
        for segment in &table.segments[common_segments..] {
            self.attribution
                .extend(segment.attribution.iter().copied().map(map_location));
            self.producers.extend(
                segment
                    .producers
                    .iter()
                    .map(|producer| Optional::from(producer.get().map(map_transform))),
            );
        }

        // copy transforms with remapped input and output lists
        for segment in &table.segments[common_segments..] {
            for transform in &segment.transforms {
                let input_start = transform.inputs.start as usize;
                let input_end = input_start + transform.inputs.len as usize;
                let output_start = transform.outputs.start as usize;
                let output_end = output_start + transform.outputs.len as usize;
                let first_input = self.inputs.len() as u32;
                let first_output = self.outputs.len() as u32;
                self.inputs.extend(
                    segment.inputs[input_start..input_end]
                        .iter()
                        .copied()
                        .map(|id| remap.destination(id)),
                );
                self.outputs.extend(
                    segment.outputs[output_start..output_end]
                        .iter()
                        .copied()
                        .map(|id| remap.destination(id)),
                );
                self.transforms.push(Transform {
                    name: transform.name,
                    inputs: EntryRange::new(first_input, transform.inputs.len),
                    outputs: EntryRange::new(first_output, transform.outputs.len),
                });
            }
        }

        remap
    }

    /// Finish the open append batch and return the complete provenance table.
    pub fn finish(mut self) -> ProvenanceTable {
        let is_empty =
            self.locations.is_empty() && self.attribution.is_empty() && self.transforms.is_empty();
        if !is_empty {
            let segment = ProvenanceSegment {
                first_provenance: self.first_provenance,
                first_location: self.first_location,
                first_transform: self.first_transform,
                locations: self.locations,
                fused_locations: self.fused_locations,
                attribution: self.attribution,
                producers: self.producers,
                transforms: self.transforms,
                inputs: self.inputs,
                outputs: self.outputs,
            };
            self.segments.push(Arc::new(segment));
        }

        ProvenanceTable {
            names: self.names,
            segments: self.segments,
        }
    }

    /// Return the next provenance id.
    fn next_provenance(&self) -> ProvenanceId {
        let offset = u32::try_from(self.attribution.len())
            .unwrap_or_else(|_| panic!("provenance table exhausted its id space"));
        let index = self
            .first_provenance
            .index()
            .checked_add(offset)
            .unwrap_or_else(|| panic!("provenance table exhausted its id space"));

        ProvenanceId::new(index)
    }

    /// Return the next location id.
    fn next_location(&self) -> LocationId {
        let offset = u32::try_from(self.locations.len())
            .unwrap_or_else(|_| panic!("provenance table exhausted its location id space"));
        let index = self
            .first_location
            .0
            .checked_add(offset)
            .unwrap_or_else(|| panic!("provenance table exhausted its location id space"));

        LocationId(index)
    }

    /// Return the next compiler transform id.
    fn next_transform(&self) -> TransformId {
        let offset = u32::try_from(self.transforms.len())
            .unwrap_or_else(|_| panic!("provenance table exhausted its transform id space"));
        let index = self
            .first_transform
            .0
            .checked_add(offset)
            .unwrap_or_else(|| panic!("provenance table exhausted its transform id space"));

        TransformId(index)
    }
}

impl From<&ProvenanceTable> for ProvenanceBuilder {
    /// Extend one finished provenance table.
    fn from(table: &ProvenanceTable) -> Self {
        Self {
            names: table.names.clone(),
            segments: table.segments.clone(),
            first_provenance: ProvenanceId::new(table.len() as u32),
            first_location: LocationId(table.location_count() as u32),
            first_transform: TransformId(table.transform_count() as u32),
            locations: Vec::new(),
            fused_locations: Vec::new(),
            attribution: Vec::new(),
            producers: Vec::new(),
            transforms: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
}
