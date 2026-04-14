/// The default contiguous arena segment width.
pub const DEFAULT_ARENA_SEGMENT_BYTES: usize = 1024 * 1024;

/// The maximum number of arena segments in one heap arena.
pub const MAX_ARENA_SEGMENTS: usize = 1 << 16;

/// The number of inline page patches per logical page map.
pub const INLINE_PAGE_PATCH_COUNT: usize = 3;

/// The default span width for local small-allocation spaces.
pub const DEFAULT_SMALL_BYTES: usize = 16 * 1024;

/// The default page width shared by spans and arena-backed payloads.
pub const DEFAULT_PAGE_BYTES: usize = 4 * 1024;

/// The default remembered-card width.
pub const DEFAULT_CARD_BYTES: usize = 256;

/// The default byte width for managed young space.
pub const DEFAULT_YOUNG_BYTES: usize = 64 * 1024;

/// The default byte width for managed references inside traced payloads.
pub const DEFAULT_MANAGED_REFERENCE_BYTES: u8 = 8;

/// The built-in default size-class table name.
pub const DEFAULT_SIZE_CLASS_TABLE_NAME: &str = "default";

/// The built-in default size classes in bytes.
pub const DEFAULT_SIZE_CLASS_BYTES: &[usize] = &[
    16, 24, 32, 48, 64, 80, 96, 128, 160, 192, 256, 320, 384, 512, 768, 1024, 1536, 2048, 3072,
    4096,
];
