mod image;
mod large;
mod location;
mod reference;
pub(crate) mod space;
mod span;

pub use image::SharedHeapSpaceImage;
pub use large::SharedHeapLargeEntryImage;
pub(crate) use large::*;
pub(crate) use location::*;
pub use reference::SharedHeapReference;
pub use space::SharedHeapSpace;
pub(crate) use space::{
    SharedHeapState, SharedLargeSpace, SharedSmallSpace, checked_slot_offset,
    checked_storage_offset, find_next_free_slot,
};
pub use span::SharedHeapSmallSpanImage;
pub(crate) use span::*;
