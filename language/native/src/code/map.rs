use destack_core::{EntryStore, SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_serde::Reflect;
use destack_source::ProvenanceId;
use serde::{Deserialize, Serialize};

use super::{
    Block, BlockId, CodeRange, CodeTrap, FrameLocation, FrameMap, FrameMapBuilder, FrameSource,
    FrameValue, ObjectFrameMap, ObjectFrameMapBuilder, ObjectTrap,
};

/// One object-local machine code extent.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ObjectCodeExtent {
    /// The object code block containing this extent.
    pub block: BlockId,
    /// The byte range relative to the block.
    pub range: CodeRange,
    /// The provenance represented by the extent.
    pub provenance: ProvenanceId,
}

/// One linked machine code extent.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct CodeExtent {
    /// The byte range relative to the linked code image.
    pub range: CodeRange,
    /// The provenance represented by the extent.
    pub provenance: ProvenanceId,
}

impl ObjectCodeExtent {
    /// Return whether this extent fits its object block.
    fn fits(self, blocks: &[Block]) -> bool {
        self.range.byte_len != 0
            && blocks
                .get(self.block.index())
                .is_some_and(|block| self.range.fits(block.byte_len() as usize))
    }

    /// Return whether this extent precedes another extent.
    fn precedes(self, other: Self) -> bool {
        if self.block == other.block {
            self.range.precedes(other.range)
        } else {
            self.block < other.block
        }
    }
}

impl CodeExtent {
    /// Return whether this extent fits its linked code image.
    fn fits(self, byte_len: usize) -> bool {
        self.range.byte_len != 0 && self.range.fits(byte_len)
    }

    /// Return whether this extent precedes another extent.
    fn precedes(self, other: Self) -> bool {
        self.range.precedes(other.range)
    }
}

/// Native code map retained by one relocatable object.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ObjectMap {
    /// Object-local machine code extents in block and byte order.
    extents: SectionSlice<ObjectCodeExtent>,
    /// Object-local native trap sites.
    traps: SectionSlice<ObjectTrap>,
    /// Object-local physical frame maps.
    frames: SectionSlice<ObjectFrameMap>,
    /// Canonical native frame values.
    values: SectionSlice<FrameValue>,
    /// Physical locations containing canonical frame values.
    locations: SectionSlice<FrameLocation>,
    /// Immutable bytes referenced by constant frame locations.
    constants: SectionSlice<u8>,
}

/// Native code map retained by linked code.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CodeMap {
    /// Linked machine code extents in byte order.
    extents: SectionSlice<CodeExtent>,
    /// Linked native trap sites in byte-offset order.
    traps: SectionSlice<CodeTrap>,
    /// Linked physical frame maps.
    frames: SectionSlice<FrameMap>,
    /// Canonical native frame values.
    values: SectionSlice<FrameValue>,
    /// Physical locations containing canonical frame values.
    locations: SectionSlice<FrameLocation>,
    /// Immutable bytes referenced by constant frame locations.
    constants: SectionSlice<u8>,
}

/// One relocatable object code map under construction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectMapBuilder {
    /// Object-local machine code extents.
    extents: Vec<ObjectCodeExtent>,
    /// Object-local native trap sites.
    traps: Vec<ObjectTrap>,
    /// Object-local physical frame maps.
    frames: Vec<ObjectFrameMapBuilder>,
    /// Immutable bytes referenced by constant frame locations.
    constants: Vec<u8>,
}

/// One linked native code map under construction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeMapBuilder {
    /// Linked machine code extents.
    extents: Vec<CodeExtent>,
    /// Linked native trap sites.
    traps: Vec<CodeTrap>,
    /// Linked physical frame maps.
    frames: Vec<FrameMapBuilder>,
    /// Immutable bytes referenced by constant frame locations.
    constants: Vec<u8>,
}

impl ObjectMap {
    /// Create one empty object code map.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return whether every object map entry fits its owning block or shared column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>, blocks: &[Block]) -> bool {
        // validate physical frame sites
        let frames = self.frames(sections);
        let frames_fit = frames.iter().all(|frame| {
            blocks
                .get(frame.block.index())
                .is_some_and(|block| frame.return_offset <= block.byte_len())
        });

        // validate sorted trap sites
        let traps = self.traps(sections);
        let traps_fit = traps.iter().all(|trap| trap.fits(blocks))
            && traps
                .windows(2)
                .all(|pair| (pair[0].block, pair[0].offset) < (pair[1].block, pair[1].offset));

        // validate sorted provenance extents
        let extents = self.extents(sections);
        let extents_fit = extents.iter().all(|extent| extent.fits(blocks))
            && extents.windows(2).all(|pair| pair[0].precedes(pair[1]));

        // validate flattened frame columns
        frames_fit
            && traps_fit
            && extents_fit
            && frame_ranges_fit(
                sections,
                frames,
                self.values,
                self.locations,
                self.constants,
                ObjectFrameMap::values_fit,
            )
    }

    /// Return object-local machine code extents in block and byte order.
    pub fn extents<'a>(&self, sections: SectionImage<'a>) -> &'a [ObjectCodeExtent] {
        sections.entries(self.extents)
    }

    /// Return the machine code extent containing one object byte offset.
    pub fn extent_at(
        &self,
        sections: SectionImage<'_>,
        block: BlockId,
        offset: u32,
    ) -> Option<ObjectCodeExtent> {
        let extents = self.extents(sections);
        let index = extents
            .partition_point(|extent| (extent.block, extent.range.offset) <= (block, offset))
            .checked_sub(1)?;
        let extent = extents[index];

        (extent.block == block && offset < extent.range.end()).then_some(extent)
    }

    /// Return the provenance containing one object byte offset.
    pub fn provenance_at(
        &self,
        sections: SectionImage<'_>,
        block: BlockId,
        offset: u32,
    ) -> Option<ProvenanceId> {
        Some(self.extent_at(sections, block, offset)?.provenance)
    }

    /// Return object-local native trap sites.
    pub fn traps<'a>(&self, sections: SectionImage<'a>) -> &'a [ObjectTrap] {
        sections.entries(self.traps)
    }

    /// Return object-local physical frame maps.
    pub fn frames<'a>(&self, sections: SectionImage<'a>) -> &'a [ObjectFrameMap] {
        sections.entries(self.frames)
    }

    /// Return canonical values in one object frame map.
    pub fn values<'a>(
        &self,
        sections: SectionImage<'a>,
        frame: ObjectFrameMap,
    ) -> &'a [FrameValue] {
        frame.values(sections.entries(self.values))
    }

    /// Return physical locations containing one canonical frame value.
    pub fn locations<'a>(
        &self,
        sections: SectionImage<'a>,
        value: FrameValue,
    ) -> &'a [FrameLocation] {
        value.locations(sections.entries(self.locations))
    }

    /// Return immutable bytes referenced by constant frame locations.
    pub fn constants<'a>(&self, sections: SectionImage<'a>) -> &'a [u8] {
        sections.entries(self.constants)
    }
}

impl CodeMap {
    /// Create one empty linked code map.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return whether every linked map entry fits the code or its shared column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>, byte_len: usize) -> bool {
        // validate sorted physical frame sites
        let frames = self.frames(sections);
        let frames_fit = frames
            .iter()
            .all(|frame| frame.return_offset as usize <= byte_len)
            && frames
                .windows(2)
                .all(|pair| pair[0].return_offset < pair[1].return_offset);

        // validate sorted trap sites
        let traps = self.traps(sections);
        let traps_fit = traps.iter().all(|trap| trap.fits(byte_len))
            && traps.windows(2).all(|pair| pair[0].offset < pair[1].offset);

        // validate sorted provenance extents
        let extents = self.extents(sections);
        let extents_fit = extents.iter().all(|extent| extent.fits(byte_len))
            && extents.windows(2).all(|pair| pair[0].precedes(pair[1]));

        // validate flattened frame columns
        frames_fit
            && traps_fit
            && extents_fit
            && frame_ranges_fit(
                sections,
                frames,
                self.values,
                self.locations,
                self.constants,
                FrameMap::values_fit,
            )
    }

    /// Return linked machine code extents in byte order.
    pub fn extents<'a>(&self, sections: SectionImage<'a>) -> &'a [CodeExtent] {
        sections.entries(self.extents)
    }

    /// Return the machine code extent containing one linked byte offset.
    pub fn extent_at(&self, sections: SectionImage<'_>, offset: u32) -> Option<CodeExtent> {
        let extents = self.extents(sections);
        let index = extents
            .partition_point(|extent| extent.range.offset <= offset)
            .checked_sub(1)?;
        let extent = extents[index];

        (offset < extent.range.end()).then_some(extent)
    }

    /// Return the provenance containing one linked machine code byte.
    pub fn provenance_at(&self, sections: SectionImage<'_>, offset: u32) -> Option<ProvenanceId> {
        Some(self.extent_at(sections, offset)?.provenance)
    }

    /// Return linked native trap sites in byte-offset order.
    pub fn traps<'a>(&self, sections: SectionImage<'a>) -> &'a [CodeTrap] {
        sections.entries(self.traps)
    }

    /// Return linked physical frame maps.
    pub fn frames<'a>(&self, sections: SectionImage<'a>) -> &'a [FrameMap] {
        sections.entries(self.frames)
    }

    /// Return one linked physical frame map by id.
    pub fn frame(&self, sections: SectionImage<'_>, id: u32) -> Option<FrameMap> {
        sections.entries(self.frames).get(id as usize).copied()
    }

    /// Return canonical values in one linked frame map.
    pub fn values<'a>(&self, sections: SectionImage<'a>, frame: FrameMap) -> &'a [FrameValue] {
        frame.values(sections.entries(self.values))
    }

    /// Return physical locations containing one canonical frame value.
    pub fn locations<'a>(
        &self,
        sections: SectionImage<'a>,
        value: FrameValue,
    ) -> &'a [FrameLocation] {
        value.locations(sections.entries(self.locations))
    }

    /// Return immutable bytes referenced by constant frame locations.
    pub fn constants<'a>(&self, sections: SectionImage<'a>) -> &'a [u8] {
        sections.entries(self.constants)
    }
}

impl ObjectMapBuilder {
    /// Create one object code map builder with its provenance extents.
    pub fn new(extents: impl IntoIterator<Item = ObjectCodeExtent>) -> Self {
        let mut extents = extents.into_iter().collect::<Vec<_>>();
        extents.sort_unstable_by_key(|extent| (extent.block, extent.range.offset));

        Self {
            extents,
            ..Self::default()
        }
    }

    /// Set object-local native trap sites.
    pub fn traps(mut self, traps: impl IntoIterator<Item = ObjectTrap>) -> Self {
        self.traps = traps.into_iter().collect();
        self.traps
            .sort_unstable_by_key(|trap| (trap.block, trap.offset));

        self
    }

    /// Set object-local physical frame maps.
    pub fn frames(mut self, frames: impl IntoIterator<Item = ObjectFrameMapBuilder>) -> Self {
        self.frames = frames.into_iter().collect();

        self
    }

    /// Set immutable frame constants.
    pub fn constants(mut self, constants: impl IntoIterator<Item = u8>) -> Self {
        self.constants = constants.into_iter().collect();

        self
    }

    /// Build this code map into object sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> ObjectMap {
        let (frames, values, locations) = build_frames(self.frames);

        ObjectMap {
            extents: sections.insert(self.extents),
            traps: sections.insert(self.traps),
            frames: sections.insert(frames),
            values: sections.insert(values),
            locations: sections.insert(locations),
            constants: sections.insert(self.constants),
        }
    }
}

impl CodeMapBuilder {
    /// Create one linked code map builder with its provenance extents.
    pub fn new(extents: impl IntoIterator<Item = CodeExtent>) -> Self {
        let mut extents = extents.into_iter().collect::<Vec<_>>();
        extents.sort_unstable_by_key(|extent| extent.range.offset);

        Self {
            extents,
            ..Self::default()
        }
    }

    /// Set linked native trap sites.
    pub fn traps(mut self, traps: impl IntoIterator<Item = CodeTrap>) -> Self {
        self.traps = traps.into_iter().collect();
        self.traps.sort_unstable_by_key(|trap| trap.offset);

        self
    }

    /// Set linked physical frame maps.
    pub fn frames(mut self, frames: impl IntoIterator<Item = FrameMapBuilder>) -> Self {
        self.frames = frames.into_iter().collect();

        self
    }

    /// Set immutable frame constants.
    pub fn constants(mut self, constants: impl IntoIterator<Item = u8>) -> Self {
        self.constants = constants.into_iter().collect();

        self
    }

    /// Build this code map into Program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> CodeMap {
        let (frames, values, locations) = build_frames(self.frames);

        CodeMap {
            extents: sections.insert(self.extents),
            traps: sections.insert(self.traps),
            frames: sections.insert(frames),
            values: sections.insert(values),
            locations: sections.insert(locations),
            constants: sections.insert(self.constants),
        }
    }
}

/// Build flattened frame values and locations for one frame-map family.
fn build_frames<B, F>(frames: Vec<B>) -> (Vec<F>, Vec<FrameValue>, Vec<FrameLocation>)
where
    B: FrameBuilder<Frame = F>,
{
    let mut values = EntryStore::new();
    let mut locations = EntryStore::new();
    let frames = frames
        .into_iter()
        .map(|frame| frame.build(&mut values, &mut locations))
        .collect();

    (frames, values.into_entries(), locations.into_entries())
}

/// Build one frame map into shared value and location columns.
trait FrameBuilder {
    /// The packed frame-map type.
    type Frame;

    /// Build one frame map.
    fn build(
        self,
        values: &mut EntryStore<FrameValue>,
        locations: &mut EntryStore<FrameLocation>,
    ) -> Self::Frame;
}

impl FrameBuilder for ObjectFrameMapBuilder {
    type Frame = ObjectFrameMap;

    fn build(
        self,
        values: &mut EntryStore<FrameValue>,
        locations: &mut EntryStore<FrameLocation>,
    ) -> Self::Frame {
        self.build(values, locations)
    }
}

impl FrameBuilder for FrameMapBuilder {
    type Frame = FrameMap;

    fn build(
        self,
        values: &mut EntryStore<FrameValue>,
        locations: &mut EntryStore<FrameLocation>,
    ) -> Self::Frame {
        self.build(values, locations)
    }
}

/// Return whether one frame-map family and its flattened columns are valid.
fn frame_ranges_fit<F: Copy>(
    sections: SectionImage<'_>,
    frames: &[F],
    values: SectionSlice<FrameValue>,
    locations: SectionSlice<FrameLocation>,
    constants: SectionSlice<u8>,
    values_fit: impl Fn(F, usize) -> bool,
) -> bool {
    let values = sections.entries(values);
    let locations = sections.entries(locations);
    let constants = sections.entries(constants);

    // require every frame and value to fit its flattened column
    let frames_fit = frames
        .iter()
        .copied()
        .all(|frame| values_fit(frame, values.len()));
    let values_fit = values
        .iter()
        .all(|value| value.locations_fit(locations.len()));
    if !frames_fit || !values_fit {
        return false;
    }

    // require constant locations to fit the mapped constant section
    locations.iter().all(|location| {
        if location.source != FrameSource::Constant {
            return true;
        }

        let Ok(start) = usize::try_from(location.source_offset) else {
            return false;
        };

        start
            .checked_add(location.byte_len as usize)
            .is_some_and(|end| end <= constants.len())
    })
}
