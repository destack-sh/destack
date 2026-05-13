/// The default allocator page size.
pub const DEFAULT_PAGE_BYTES: usize = 8 * 1024;

/// The default allocator chunk size.
pub const DEFAULT_ALLOCATOR_CHUNK_BYTES: usize = if cfg!(target_arch = "wasm32") {
    512 * 1024
} else if cfg!(target_pointer_width = "64") && !cfg!(target_os = "windows") {
    64 * 1024 * 1024
} else {
    4 * 1024 * 1024
};
