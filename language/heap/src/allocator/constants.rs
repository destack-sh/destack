/// The standard allocator page size.
pub const DEFAULT_PAGE_BYTES: usize = 8 * 1024;

/// The standard allocator arena size.
pub const DEFAULT_ALLOCATOR_ARENA_BYTES: usize = if cfg!(target_arch = "wasm32") {
    512 * 1024
} else if cfg!(target_pointer_width = "64")
    && !cfg!(target_os = "windows")
    && !(cfg!(target_os = "ios") && cfg!(target_arch = "aarch64"))
{
    64 * 1024 * 1024
} else {
    4 * 1024 * 1024
};

/// The supported user virtual address width for allocator arena lookup.
pub(crate) const ALLOCATOR_ADDRESS_BITS: usize =
    if cfg!(target_os = "ios") && cfg!(target_arch = "aarch64") {
        40
    } else if cfg!(target_pointer_width = "64") {
        48
    } else {
        usize::BITS as usize
    };

/// The number of arena pointers per lazily allocated arena-table chunk.
pub(crate) const ARENA_TABLE_CHUNK_LEN: usize = 1 << 12;
