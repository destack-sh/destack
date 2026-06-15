/// Magic prefix for on disk artifact payload blobs.
pub const ARTIFACT_BLOB_MAGIC: [u8; 4] = *b"DSCH";
/// Limit cached blob size to avoid excessive memory usage.
pub const CACHE_BLOB_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
/// Number of bytes used to encode one header length prefix.
pub const ARTIFACT_BLOB_HEADER_LENGTH_BYTES: usize = 4;
