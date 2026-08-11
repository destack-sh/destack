/// Current trace format version.
pub const TRACE_FORMAT_VERSION: u32 = 1;

/// Default maximum number of entries in one trace chunk.
pub const TRACE_DEFAULT_MAX_ENTRIES_PER_CHUNK: u32 = 1024;

/// Default maximum byte length of one trace chunk.
pub const TRACE_DEFAULT_MAX_CHUNK_SIZE_BYTES: u64 = 4 * 1024 * 1024;

/// Byte length of one trace entry length prefix.
pub(super) const TRACE_ENTRY_LENGTH_BYTES: usize = std::mem::size_of::<u32>();

/// Byte length of one trace entry tag.
pub(super) const TRACE_ENTRY_TAG_BYTES: usize = std::mem::size_of::<u8>();
