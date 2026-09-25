use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

/// A global identity, object-local before linking and Program-local afterward.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct GlobalId(pub u32);

impl GlobalId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

const _: () = assert!(size_of::<GlobalId>() == 4);
