use destack_core::{EntryRange, EntryStore, SectionEntry};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Physical native projection of one canonical Program frame state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameMap {
    /// Object-local function index before linking, or Program function id after linking.
    pub function: u32,
    /// Return-address byte offset from the containing function body.
    pub return_offset: u32,
    /// Signed byte offset from the frame marker to the native frame pointer.
    pub frame_pointer_offset: i32,
    /// Object-local frame-state index before linking, or Program frame-state id after linking.
    pub state: u32,
    /// Program operation resumed after restoring this frame.
    pub resume: u32,
    /// Canonical values in Program frame slot order.
    values: EntryRange<FrameValue>,
}

impl FrameMap {
    /// Return whether this frame's value range fits its shared column.
    pub(super) fn values_fit(self, values: usize) -> bool {
        self.values.fits(values)
    }

    /// Return canonical values in Program frame slot order.
    pub fn values(self, values: &[FrameValue]) -> &[FrameValue] {
        self.values.slice(values)
    }
}

/// Mutable native frame map before section packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMapBuilder {
    /// Function identity selected by the containing representation.
    function: u32,
    /// Return-address byte offset from the containing function body.
    return_offset: u32,
    /// Signed byte offset from the frame marker to the native frame pointer.
    frame_pointer_offset: i32,
    /// Frame-state identity selected by the containing representation.
    state: u32,
    /// Program operation resumed after restoring this frame.
    resume: u32,
    /// Canonical values in Program frame slot order.
    values: Vec<FrameValueBuilder>,
}

impl FrameMapBuilder {
    /// Create one native frame map builder.
    pub const fn new(
        function: u32,
        return_offset: u32,
        frame_pointer_offset: i32,
        state: u32,
        resume: u32,
    ) -> Self {
        Self {
            function,
            return_offset,
            frame_pointer_offset,
            state,
            resume,
            values: Vec::new(),
        }
    }

    /// Set canonical values in Program frame slot order.
    pub fn values(mut self, values: impl IntoIterator<Item = FrameValueBuilder>) -> Self {
        self.values = values.into_iter().collect();

        self
    }

    /// Pack this frame map into flattened code map storage.
    pub(super) fn build(
        self,
        values: &mut EntryStore<FrameValue>,
        locations: &mut EntryStore<FrameLocation>,
    ) -> FrameMap {
        let entries = self
            .values
            .into_iter()
            .map(|value| value.build(locations))
            .collect::<Vec<_>>();

        FrameMap {
            function: self.function,
            return_offset: self.return_offset,
            frame_pointer_offset: self.frame_pointer_offset,
            state: self.state,
            resume: self.resume,
            values: values.append(entries),
        }
    }
}

/// Physical pieces containing one canonical frame value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameValue {
    /// Physical pieces in canonical byte order.
    locations: EntryRange<FrameLocation>,
}

impl FrameValue {
    /// Return whether this value's location range fits its shared column.
    pub(super) fn locations_fit(self, locations: usize) -> bool {
        self.locations.fits(locations)
    }

    /// Return physical pieces in canonical byte order.
    pub fn locations(self, locations: &[FrameLocation]) -> &[FrameLocation] {
        self.locations.slice(locations)
    }
}

/// Mutable physical pieces for one canonical frame value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameValueBuilder {
    /// Physical pieces in canonical byte order.
    locations: Vec<FrameLocation>,
}

impl FrameValueBuilder {
    /// Create one empty native frame value builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set physical pieces in canonical byte order.
    pub fn locations(mut self, locations: impl IntoIterator<Item = FrameLocation>) -> Self {
        self.locations = locations.into_iter().collect();

        self
    }

    /// Pack this value into flattened location storage.
    fn build(self, locations: &mut EntryStore<FrameLocation>) -> FrameValue {
        FrameValue {
            locations: locations.append(self.locations),
        }
    }
}

/// One physical piece of a canonical native frame value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameLocation {
    /// Physical source containing this piece.
    pub source: FrameSource,
    /// Signed byte offset selected by the source.
    pub source_offset: i32,
    /// Byte offset inside the canonical value.
    pub target_offset: u32,
    /// Byte length of this piece.
    pub byte_len: u32,
}

impl FrameLocation {
    /// Create one physical native frame location.
    pub const fn new(
        source: FrameSource,
        source_offset: i32,
        target_offset: u32,
        byte_len: u32,
    ) -> Self {
        Self {
            source,
            source_offset,
            target_offset,
            byte_len,
        }
    }
}

/// Physical native source containing one frame value piece.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum FrameSource {
    /// Native stack bytes addressed relative to the frame marker.
    Stack = 0,
    /// Immutable bytes addressed inside the native code map constant section.
    Constant = 1,
}
