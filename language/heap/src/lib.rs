mod allocator;
mod core;
mod local;
mod shared;

#[cfg(test)]
pub(crate) use allocator::test::*;
pub use allocator::{
    Allocator, Bitmap, DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES, DEFAULT_MAX_SMALL_ALLOCATION_BYTES,
    DEFAULT_PAGE_SIZE_BYTES, PageId, PageRun, SizeClass, SizeClassPolicy, SizeClassTable, SpanSlot,
};
pub use core::*;
pub use local::*;
pub use shared::*;
