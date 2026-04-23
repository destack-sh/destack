/// The standard small-span width for size-classed allocation.
pub(crate) const DEFAULT_SMALL_BYTES: usize = 16 * 1024;

/// The standard remembered-card width for local write tracking.
pub(crate) const DEFAULT_CARD_BYTES: usize = 256;

/// The standard small young-space width for worker-local heaps.
pub(crate) const DEFAULT_YOUNG_BYTES: usize = 64 * 1024;

/// The standard nursery bypass threshold for larger payloads.
pub(crate) const DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES: usize = 4 * 1024;

/// The standard alignment for configured small-allocation classes.
pub(crate) const DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES: usize = 8;
