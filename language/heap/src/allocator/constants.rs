use std::sync::{Arc, LazyLock};

use super::class::SizeClass;

/// The default allocator page size.
pub const DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES: usize = 8 * 1024;

/// The default allocator chunk size.
pub const DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES: usize = if cfg!(target_arch = "wasm32") {
    512 * 1024
} else if cfg!(target_pointer_width = "64") && !cfg!(target_os = "windows") {
    64 * 1024 * 1024
} else {
    4 * 1024 * 1024
};

/// The default small-block class payload sizes and span page counts.
pub(crate) const DEFAULT_SIZE_CLASSES: [(usize, usize); 67] = [
    (8, 1),
    (16, 1),
    (24, 1),
    (32, 1),
    (48, 1),
    (64, 1),
    (80, 1),
    (96, 1),
    (112, 1),
    (128, 1),
    (144, 1),
    (160, 1),
    (176, 1),
    (192, 1),
    (208, 1),
    (224, 1),
    (240, 1),
    (256, 1),
    (288, 1),
    (320, 1),
    (352, 1),
    (384, 1),
    (416, 1),
    (448, 1),
    (480, 1),
    (512, 1),
    (576, 1),
    (640, 1),
    (704, 1),
    (768, 1),
    (896, 1),
    (1024, 1),
    (1152, 1),
    (1280, 1),
    (1408, 2),
    (1536, 1),
    (1792, 2),
    (2048, 1),
    (2304, 2),
    (2688, 1),
    (3072, 3),
    (3200, 2),
    (3456, 3),
    (4096, 1),
    (4864, 3),
    (5376, 2),
    (6144, 3),
    (6528, 4),
    (6784, 5),
    (6912, 6),
    (8192, 1),
    (9472, 7),
    (9728, 6),
    (10240, 5),
    (10880, 4),
    (12288, 3),
    (13568, 5),
    (14336, 7),
    (16384, 2),
    (18432, 9),
    (19072, 7),
    (20480, 5),
    (21760, 8),
    (24576, 3),
    (27264, 10),
    (28672, 7),
    (32768, 4),
];

/// The largest payload routed through the default small-block table.
pub const DEFAULT_MAX_SMALL_ALLOCATION_BYTES: usize =
    DEFAULT_SIZE_CLASSES[DEFAULT_SIZE_CLASSES.len() - 1].0;

/// Shared storage for the default size-class table.
pub(crate) static DEFAULT_SIZE_CLASS_TABLE_CLASSES: LazyLock<Arc<[SizeClass]>> =
    LazyLock::new(|| {
        DEFAULT_SIZE_CLASSES
            .iter()
            .map(|&(bytes, span_page_count)| {
                let span_size_bytes = span_page_count * DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES;

                SizeClass::with_span_size_bytes(bytes, span_size_bytes)
            })
            .collect::<Vec<_>>()
            .into()
    });
