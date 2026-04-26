use crate::SizeClassPolicy;
use crate::allocator::DEFAULT_PAGE_BYTES;

/// The standard small-span width for size-classed allocation.
pub const DEFAULT_SMALL_BYTES: usize = DEFAULT_PAGE_BYTES * 10;

/// The standard remembered-card width for local write tracking.
pub(crate) const DEFAULT_CARD_BYTES: usize = DEFAULT_PAGE_BYTES / 32;

/// The standard small young-space width for worker-heaps.
pub const DEFAULT_YOUNG_BYTES: usize = 64 * 1024;

/// The standard nursery bypass threshold for larger payloads.
pub const DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES: usize = SizeClassPolicy::DEFAULT_MAX_BYTES;

/// The standard alignment for configured small-allocation classes.
pub const DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES: usize =
    SizeClassPolicy::DEFAULT_ALIGNMENT_BYTES;
