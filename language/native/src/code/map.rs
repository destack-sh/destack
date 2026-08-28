use destack_core::{
    EntryStore, SectionBuilder, SectionEntry, SectionImage, SectionImageError, SectionSlice,
};
use destack_serde::Reflect;
use destack_source::ProvenanceId;
use serde::{Deserialize, Serialize};

use super::{
    Block, BlockId, CodeRange, CodeTrap, FrameLocation, FrameMap, FrameMapBuilder, FrameValue,
    ObjectFrameMap, ObjectFrameMapBuilder, ObjectTrap,
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
    /// Validate this extent against its object block.
    fn validate(self, blocks: &[Block]) -> Result<(), SectionImageError> {
        if self.range.byte_len == 0 {
            return Err(SectionImageError::InvalidRange);
        }
        let Some(block) = blocks.get(self.block.index()) else {
            return Err(SectionImageError::InvalidReference);
        };

        self.range.validate(block.byte_len() as usize)
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
    /// Validate this extent against its linked code image.
    fn validate(self, byte_len: usize) -> Result<(), SectionImageError> {
        if self.range.byte_len == 0 {
            return Err(SectionImageError::InvalidRange);
        }

        self.range.validate(byte_len)
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

    /// Validate every object map entry against its owning block or shared column.
    pub(super) fn validate(
        &self,
        sections: SectionImage<'_>,
        blocks: &[Block],
    ) -> Result<(), SectionImageError> {
        // validate physical frame sites
        let frames = self.frames(sections);
        for frame in frames {
            let Some(block) = blocks.get(frame.block.index()) else {
                return Err(SectionImageError::InvalidReference);
            };
            if frame.return_offset > block.byte_len() {
                return Err(SectionImageError::InvalidRange);
            }
        }

        // validate sorted trap sites
        let traps = self.traps(sections);
        for trap in traps {
            trap.validate(blocks)?;
        }
        if !traps
            .windows(2)
            .all(|pair| (pair[0].block, pair[0].offset) < (pair[1].block, pair[1].offset))
        {
            return Err(SectionImageError::InvalidOrder);
        }

        // validate sorted provenance extents
        let extents = self.extents(sections);
        for extent in extents {
            extent.validate(blocks)?;
        }
        if !extents.windows(2).all(|pair| pair[0].precedes(pair[1])) {
            return Err(SectionImageError::InvalidOrder);
        }

        // validate flattened frame columns
        let values = sections.entries(self.values);
        let locations = sections.entries(self.locations);
        let constants = sections.entries(self.constants);
        for frame in frames {
            frame.validate(values.len())?;
        }
        for value in values {
            value.validate(locations.len())?;
        }
        for location in locations {
            location.validate(constants.len())?;
        }

        Ok(())
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

    /// Validate every linked map entry against the code or its shared column.
    pub(super) fn validate(
        &self,
        sections: SectionImage<'_>,
        byte_len: usize,
    ) -> Result<(), SectionImageError> {
        // validate sorted physical frame sites
        let frames = self.frames(sections);
        if frames
            .iter()
            .any(|frame| frame.return_offset as usize > byte_len)
        {
            return Err(SectionImageError::InvalidRange);
        }
        if !frames
            .windows(2)
            .all(|pair| pair[0].return_offset < pair[1].return_offset)
        {
            return Err(SectionImageError::InvalidOrder);
        }

        // validate sorted trap sites
        let traps = self.traps(sections);
        for trap in traps {
            trap.validate(byte_len)?;
        }
        if !traps.windows(2).all(|pair| pair[0].offset < pair[1].offset) {
            return Err(SectionImageError::InvalidOrder);
        }

        // validate sorted provenance extents
        let extents = self.extents(sections);
        for extent in extents {
            extent.validate(byte_len)?;
        }
        if !extents.windows(2).all(|pair| pair[0].precedes(pair[1])) {
            return Err(SectionImageError::InvalidOrder);
        }

        // validate flattened frame columns
        let values = sections.entries(self.values);
        let locations = sections.entries(self.locations);
        let constants = sections.entries(self.constants);
        for frame in frames {
            frame.validate(values.len())?;
        }
        for value in values {
            value.validate(locations.len())?;
        }
        for location in locations {
            location.validate(constants.len())?;
        }

        Ok(())
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
