use std::sync::{Arc, LazyLock};

use super::class::SizeClass;

/// The default allocator page size.
pub const DEFAULT_PAGE_SIZE_BYTES: usize = 8 * 1024;

/// The default allocator chunk size.
pub const DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES: usize = if cfg!(target_arch = "wasm32") {
    512 * 1024
} else if cfg!(target_pointer_width = "64") && !cfg!(target_os = "windows") {
    64 * 1024 * 1024
} else {
    4 * 1024 * 1024
};

/// The default small-allocation class payload sizes.
pub(crate) const DEFAULT_SIZE_CLASS_BYTES: [usize; 67] = [
    8, 16, 24, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240, 256, 288, 320, 352,
    384, 416, 448, 480, 512, 576, 640, 704, 768, 896, 1024, 1152, 1280, 1408, 1536, 1792, 2048,
    2304, 2688, 3072, 3200, 3456, 4096, 4864, 5376, 6144, 6528, 6784, 6912, 8192, 9472, 9728,
    10240, 10880, 12288, 13568, 14336, 16384, 18432, 19072, 20480, 21760, 24576, 27264, 28672,
    32768,
];

/// The largest payload routed through the default small-allocation table.
pub const DEFAULT_MAX_SMALL_ALLOCATION_BYTES: usize =
    DEFAULT_SIZE_CLASS_BYTES[DEFAULT_SIZE_CLASS_BYTES.len() - 1];

/// The page width used by the default size-class table.
pub(crate) const DEFAULT_SIZE_CLASS_PAGE_SIZE_BYTES: usize = DEFAULT_PAGE_SIZE_BYTES;

/// The default small-allocation class span page counts.
pub(crate) const DEFAULT_SIZE_CLASS_SPAN_PAGE_COUNTS: [usize; 67] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 2, 1, 2, 1, 2, 1, 3, 2, 3, 1, 3, 2, 3, 4, 5, 6, 1, 7, 6, 5, 4, 3, 5, 7, 2, 9, 7, 5, 8, 3,
    10, 7, 4,
];

/// Shared storage for the default size-class table.
pub(crate) static DEFAULT_SIZE_CLASS_TABLE_CLASSES: LazyLock<Arc<[SizeClass]>> =
    LazyLock::new(|| {
        DEFAULT_SIZE_CLASS_BYTES
            .iter()
            .zip(DEFAULT_SIZE_CLASS_SPAN_PAGE_COUNTS)
            .map(|(&bytes, span_page_count)| {
                let span_size_bytes = span_page_count * DEFAULT_SIZE_CLASS_PAGE_SIZE_BYTES;

                SizeClass::with_span_size_bytes(bytes, span_size_bytes)
            })
            .collect::<Vec<_>>()
            .into()
    });
