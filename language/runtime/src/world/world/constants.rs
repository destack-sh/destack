/// Current World snapshot format version.
pub const WORLD_SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// The virtual byte capacity reserved for one 64-bit World memory map.
#[cfg(target_pointer_width = "64")]
pub(super) const WORLD_MEMORY_MAP_SIZE_BYTES: usize = 4 * 1024 * 1024 * 1024 * 1024;

/// The virtual byte capacity reserved for one narrow-pointer World memory map.
#[cfg(not(target_pointer_width = "64"))]
pub(super) const WORLD_MEMORY_MAP_SIZE_BYTES: usize = 1024 * 1024 * 1024;
