mod allocator;
mod core;
mod local;
mod shared;

#[cfg(test)]
pub(crate) use allocator::test::*;
pub use allocator::{
    Allocator, Bitmap, DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES, DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES,
    DEFAULT_MAX_SMALL_ALLOCATION_BYTES, PageId, PageSpan, SizeClass, SizeClassPolicy,
    SizeClassTable, Slot,
};
pub use core::*;
pub use local::*;
pub use shared::*;
