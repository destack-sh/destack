use destack_core::{EntryStore, SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{FrameLocation, FrameMap, FrameMapBuilder, FrameSource, FrameValue};

/// Native frame maps for collection, inspection, and deoptimization.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CodeMap {
    /// Physical native frame maps in dense id order.
    frames: SectionSlice<FrameMap>,
    /// Canonical native frame values.
    values: SectionSlice<FrameValue>,
    /// Physical locations containing canonical frame values.
    locations: SectionSlice<FrameLocation>,
    /// Immutable bytes referenced by constant frame locations.
    constants: SectionSlice<u8>,
}

/// Build-time native code map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeMapBuilder {
    /// Physical native frame maps in dense id order.
    frames: Vec<FrameMapBuilder>,
    /// Immutable bytes referenced by constant frame locations.
    constants: Vec<u8>,
}

impl CodeMapBuilder {
    /// Create an empty native code map builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set physical native frame maps in dense id order.
    pub fn frames(mut self, frames: impl IntoIterator<Item = FrameMapBuilder>) -> Self {
        self.frames = frames.into_iter().collect();

        self
    }

    /// Set immutable bytes referenced by constant frame locations.
    pub fn constants(mut self, constants: impl IntoIterator<Item = u8>) -> Self {
        self.constants = constants.into_iter().collect();

        self
    }

    /// Build this code map into program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> CodeMap {
        let mut values = EntryStore::<FrameValue>::new();
        let mut locations = EntryStore::<FrameLocation>::new();
        let frames = self
            .frames
            .into_iter()
            .map(|frame| frame.build(&mut values, &mut locations))
            .collect::<Vec<_>>();

        CodeMap {
            frames: sections.insert(frames),
            values: sections.insert(values.into_entries()),
            locations: sections.insert(locations.into_entries()),
            constants: sections.insert(self.constants),
        }
    }
}

impl CodeMap {
    /// Create one empty native code map.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return whether every relative frame-map range fits its shared column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let frames = sections.entries(self.frames);
        let values = sections.entries(self.values);
        let locations = sections.entries(self.locations);
        let constants = sections.entries(self.constants);

        // require every frame and value to fit its flattened column
        let frames_fit = frames.iter().all(|frame| frame.values_fit(values.len()));
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

    /// Return physical native frame maps.
    pub fn frames<'a>(&self, sections: SectionImage<'a>) -> &'a [FrameMap] {
        sections.entries(self.frames)
    }

    /// Return one physical native frame map by id.
    pub fn frame(&self, sections: SectionImage<'_>, id: u32) -> Option<FrameMap> {
        sections.entries(self.frames).get(id as usize).copied()
    }

    /// Return canonical values in one native frame map.
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
