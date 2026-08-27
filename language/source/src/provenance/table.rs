use std::num::NonZeroU32;
use std::sync::Arc;

use destack_core::{EntryRange, Optional, SectionEntry, StringId, StringPool};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::Span;

use super::ProvenanceBuilder;

/// Identifies the provenance of one logical construct.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct ProvenanceId(NonZeroU32);

impl ProvenanceId {
    /// Create a provenance id from its zero based table index.
    pub const fn new(index: u32) -> Self {
        let Some(raw) = index.checked_add(1) else {
            panic!("provenance table exhausted its id space");
        };
        let Some(raw) = NonZeroU32::new(raw) else {
            unreachable!();
        };

        Self(raw)
    }

    /// Return the zero based table index.
    pub const fn index(self) -> u32 {
        self.0.get() - 1
    }
}

/// One location in a provenance table.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct LocationId(pub u32);

/// One transform in a provenance table.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct TransformId(pub u32);

/// One direct source attribution.
#[repr(C, u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum Location {
    /// One authored construct.
    Authored {
        /// The complete authored range.
        span: Span,
        /// The preferred diagnostic and source map range.
        primary: Optional<Span>,
    },
    /// One source location instantiated at another source site.
    Expanded {
        /// The location being expanded.
        source: LocationId,
        /// The site causing the expansion.
        site: LocationId,
    },
    /// Several source locations combined into one attribution.
    Fused(EntryRange<LocationId>),
    /// Compiler generated code without one direct authored location.
    Generated,
}

/// One compiler transform connecting any number of inputs and outputs.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Transform {
    /// The stable transform name.
    pub name: StringId,
    /// The causal input provenance.
    pub inputs: EntryRange<ProvenanceId>,
    /// The produced provenance.
    pub outputs: EntryRange<ProvenanceId>,
}

/// One immutable append batch in a provenance table.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ProvenanceSegment {
    /// The first provenance id allocated by this segment.
    pub(super) first_provenance: ProvenanceId,
    /// The first location allocated by this segment.
    pub(super) first_location: LocationId,
    /// The first transform allocated by this segment.
    pub(super) first_transform: TransformId,
    /// Locations allocated by this segment.
    pub(super) locations: Vec<Location>,
    /// Locations referenced by fused locations.
    pub(super) fused_locations: Vec<LocationId>,
    /// Direct attribution indexed by provenance id.
    pub(super) attribution: Vec<LocationId>,
    /// Producing transforms indexed by provenance id.
    pub(super) producers: Vec<Optional<TransformId>>,
    /// Transforms allocated by this segment.
    pub(super) transforms: Vec<Transform>,
    /// Transform inputs allocated by this segment.
    pub(super) inputs: Vec<ProvenanceId>,
    /// Transform outputs allocated by this segment.
    pub(super) outputs: Vec<ProvenanceId>,
}

impl ProvenanceSegment {
    /// Return the first provenance id in this segment.
    pub fn first_provenance(&self) -> ProvenanceId {
        self.first_provenance
    }

    /// Return the first location id in this segment.
    pub fn first_location(&self) -> LocationId {
        self.first_location
    }

    /// Return the first transform id in this segment.
    pub fn first_transform(&self) -> TransformId {
        self.first_transform
    }

    /// Return the locations allocated by this segment.
    pub fn locations(&self) -> &[Location] {
        &self.locations
    }

    /// Return the packed locations referenced by fused locations.
    pub fn fused_locations(&self) -> &[LocationId] {
        &self.fused_locations
    }

    /// Return the direct attribution column.
    pub fn attribution(&self) -> &[LocationId] {
        &self.attribution
    }

    /// Return the producing transform column.
    pub fn producers(&self) -> &[Optional<TransformId>] {
        &self.producers
    }

    /// Return the transforms allocated by this segment.
    pub fn transforms(&self) -> &[Transform] {
        &self.transforms
    }

    /// Return the packed transform inputs.
    pub fn inputs(&self) -> &[ProvenanceId] {
        &self.inputs
    }

    /// Return the packed transform outputs.
    pub fn outputs(&self) -> &[ProvenanceId] {
        &self.outputs
    }

    /// Return the dense index of one provenance id in this segment.
    fn provenance_index(&self, id: ProvenanceId) -> Option<usize> {
        let index = id.index().checked_sub(self.first_provenance.index())? as usize;

        (index < self.attribution.len()).then_some(index)
    }

    /// Return the dense index of one location id in this segment.
    fn location_index(&self, id: LocationId) -> Option<usize> {
        let index = id.0.checked_sub(self.first_location.0)? as usize;

        (index < self.locations.len()).then_some(index)
    }

    /// Return the dense index of one transform id in this segment.
    fn transform_index(&self, id: TransformId) -> Option<usize> {
        let index = id.0.checked_sub(self.first_transform.0)? as usize;

        (index < self.transforms.len()).then_some(index)
    }
}

/// Source attribution and transform history for one compilation artifact.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ProvenanceTable {
    /// Stable transform names.
    pub(super) names: StringPool,
    /// Immutable append batches in allocation order.
    pub(super) segments: Vec<Arc<ProvenanceSegment>>,
}

impl ProvenanceTable {
    /// Create an empty provenance table.
    pub fn new() -> Self {
        Self {
            names: StringPool::new(),
            segments: Vec::new(),
        }
    }

    /// Create a builder for an empty provenance table.
    pub fn build() -> ProvenanceBuilder {
        ProvenanceBuilder::new()
    }

    /// Extend this provenance table with one mutable append batch.
    pub fn extend(&self) -> ProvenanceBuilder {
        ProvenanceBuilder::from(self)
    }

    /// Return the stable transform names.
    pub fn names(&self) -> &StringPool {
        &self.names
    }

    /// Return the immutable append batches.
    pub fn segments(&self) -> &[Arc<ProvenanceSegment>] {
        &self.segments
    }

    /// Return the number of provenance ids.
    pub fn len(&self) -> usize {
        self.segments
            .last()
            .map(|segment| segment.first_provenance.index() as usize + segment.attribution.len())
            .unwrap_or(0)
    }

    /// Return whether the table contains no provenance.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the direct attribution of one provenance id.
    pub fn attribution(&self, id: ProvenanceId) -> Option<LocationId> {
        let segment = Self::provenance_segment_in(&self.segments, id)?;
        let index = segment.provenance_index(id)?;

        segment.attribution.get(index).copied()
    }

    /// Return the transform that produced one provenance id.
    pub fn producer(&self, id: ProvenanceId) -> Option<TransformId> {
        let segment = Self::provenance_segment_in(&self.segments, id)?;
        let index = segment.provenance_index(id)?;

        segment.producers.get(index)?.get()
    }

    /// Return one location.
    pub fn location(&self, id: LocationId) -> Option<Location> {
        let segment = self.location_segment(id)?;
        let index = segment.location_index(id)?;

        segment.locations.get(index).copied()
    }

    /// Return the locations represented by one fused location.
    pub fn fused(&self, id: LocationId) -> Option<&[LocationId]> {
        let segment = self.location_segment(id)?;
        let index = segment.location_index(id)?;
        let Location::Fused(locations) = segment.locations.get(index)? else {
            return None;
        };
        let start = locations.start as usize;
        let end = start + locations.len as usize;

        segment.fused_locations.get(start..end)
    }

    /// Return one compiler transform.
    pub fn transform(&self, id: TransformId) -> Option<Transform> {
        let segment = self.transform_segment(id)?;
        let index = segment.transform_index(id)?;

        segment.transforms.get(index).copied()
    }

    /// Return the causal inputs of one compiler transform.
    pub fn inputs(&self, id: TransformId) -> Option<&[ProvenanceId]> {
        let segment = self.transform_segment(id)?;
        let index = segment.transform_index(id)?;
        let transform = segment.transforms.get(index)?;
        let start = transform.inputs.start as usize;
        let end = start + transform.inputs.len as usize;

        segment.inputs.get(start..end)
    }

    /// Return the outputs produced by one compiler transform.
    pub fn outputs(&self, id: TransformId) -> Option<&[ProvenanceId]> {
        let segment = self.transform_segment(id)?;
        let index = segment.transform_index(id)?;
        let transform = segment.transforms.get(index)?;
        let start = transform.outputs.start as usize;
        let end = start + transform.outputs.len as usize;

        segment.outputs.get(start..end)
    }

    /// Return the stable name of one compiler transform.
    pub fn name(&self, id: TransformId) -> Option<&str> {
        let transform = self.transform(id)?;

        self.names.get_maybe(transform.name)
    }

    /// Return the complete authored span represented by one provenance id.
    pub fn span(&self, id: ProvenanceId) -> Option<Span> {
        let location = self.attribution(id)?;

        self.location_span(location, |span, _| Some(span))
    }

    /// Return the primary authored span represented by one provenance id.
    pub fn primary_span(&self, id: ProvenanceId) -> Option<Span> {
        let location = self.attribution(id)?;

        self.location_span(location, |_, primary| primary.get())
    }

    /// Return the append batch that contains one provenance id.
    pub(super) fn provenance_segment_in(
        segments: &[Arc<ProvenanceSegment>],
        id: ProvenanceId,
    ) -> Option<&ProvenanceSegment> {
        let index = segments
            .partition_point(|segment| segment.first_provenance <= id)
            .checked_sub(1)?;
        let segment = &segments[index];
        let end = segment.first_provenance.index() as usize + segment.attribution.len();

        ((id.index() as usize) < end).then_some(segment)
    }

    /// Return the append batch that contains one location.
    fn location_segment(&self, id: LocationId) -> Option<&ProvenanceSegment> {
        let index = self
            .segments
            .partition_point(|segment| segment.first_location <= id)
            .checked_sub(1)?;
        let segment = &self.segments[index];
        let end = segment.first_location.0 as usize + segment.locations.len();

        ((id.0 as usize) < end).then_some(segment)
    }

    /// Return the append batch that contains one compiler transform.
    fn transform_segment(&self, id: TransformId) -> Option<&ProvenanceSegment> {
        let index = self
            .segments
            .partition_point(|segment| segment.first_transform <= id)
            .checked_sub(1)?;
        let segment = &self.segments[index];
        let end = segment.first_transform.0 as usize + segment.transforms.len();

        ((id.0 as usize) < end).then_some(segment)
    }

    /// Return one span shared by every represented authored location.
    fn location_span(
        &self,
        id: LocationId,
        authored: impl Fn(Span, Optional<Span>) -> Option<Span>,
    ) -> Option<Span> {
        let mut pending = vec![id];
        let mut common = None;

        // project nested expansions and fused locations without recursion
        while let Some(id) = pending.pop() {
            match self.location(id)? {
                Location::Authored { span, primary } => {
                    let span = authored(span, primary)?;
                    if common.is_some_and(|common| common != span) {
                        return None;
                    }
                    common = Some(span);
                }
                Location::Expanded { source, .. } => pending.push(source),
                Location::Fused(_) => pending.extend(self.fused(id)?.iter().rev().copied()),
                Location::Generated => return None,
            }
        }

        common
    }

    /// Return the number of locations.
    pub(super) fn location_count(&self) -> usize {
        self.segments
            .last()
            .map(|segment| segment.first_location.0 as usize + segment.locations.len())
            .unwrap_or(0)
    }

    /// Return the number of compiler transforms.
    pub(super) fn transform_count(&self) -> usize {
        self.segments
            .last()
            .map(|segment| segment.first_transform.0 as usize + segment.transforms.len())
            .unwrap_or(0)
    }
}
