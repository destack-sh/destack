use crate::SizeClassPolicy;
use crate::allocator::{DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES, DEFAULT_MAX_SMALL_ALLOCATION_BYTES};

/// The standard small-span width for size-classed block.
pub const DEFAULT_SMALL_SIZE_BYTES: usize = DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES * 10;

/// The standard young-space width for worker heaps.
pub const DEFAULT_YOUNG_SIZE_BYTES: usize = 1024 * 1024;

/// The standard shared small-span width for worker block caches.
pub const DEFAULT_SHARED_SMALL_SIZE_BYTES: usize = DEFAULT_YOUNG_SIZE_BYTES;

/// The standard virtual byte capacity for one forkable heap space.
pub const DEFAULT_ADDRESS_SPACE_SIZE_BYTES: usize = if cfg!(target_pointer_width = "64") {
    64 * 1024 * 1024 * 1024
} else {
    256 * 1024 * 1024
};

/// The standard remembered-card width for local write tracking.
pub(crate) const DEFAULT_CARD_SIZE_BYTES: usize = DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES / 32;

/// The standard nursery bypass threshold for larger payloads.
pub const DEFAULT_MAX_HEAP_YOUNG_ALLOCATION_SIZE_BYTES: usize = DEFAULT_MAX_SMALL_ALLOCATION_BYTES;

/// The standard alignment for configured small-block classes.
pub const DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES: usize =
    SizeClassPolicy::DEFAULT_ALIGNMENT_BYTES;
