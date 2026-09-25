use serde::{Deserialize, Serialize};
use tspp_core::{EntryRange, EntryStore, SectionEntry};
use tspp_serde::Reflect;

/// Physical WebAssembly projection of one canonical frame state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameMap {
    /// Materialized slots in canonical Program frame slot order.
    pub(super) slots: EntryRange<FrameSlot>,
}

/// Mutable WebAssembly frame map before Program section packing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameMapBuilder {
    /// Materialized slots in canonical Program frame slot order.
    slots: Vec<FrameSlot>,
}

impl FrameMapBuilder {
    /// Create one empty WebAssembly frame map builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set materialized slots in canonical Program frame slot order.
    pub fn slots(mut self, slots: impl IntoIterator<Item = FrameSlot>) -> Self {
        self.slots = slots.into_iter().collect();

        self
    }

    /// Build this frame map into flattened slot storage.
    pub(super) fn build(self, slots: &mut EntryStore<FrameSlot>) -> FrameMap {
        FrameMap {
            slots: slots.append(self.slots),
        }
    }
}

/// One canonical value materialized in WebAssembly activation memory.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameSlot {
    /// Byte offset from the WebAssembly activation frame base.
    pub offset: u32,
}

impl FrameSlot {
    /// Create one materialized WebAssembly frame slot.
    pub const fn new(offset: u32) -> Self {
        Self { offset }
    }
}
