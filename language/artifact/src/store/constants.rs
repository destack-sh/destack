/// Magic prefix for on disk artifact payload blobs.
pub const ARTIFACT_BLOB_MAGIC: [u8; 4] = *b"DSCH";
/// File extension used for immutable artifact record segments.
pub const ARTIFACT_SEGMENT_EXTENSION: &str = "segment";
/// Number of bytes used to encode one header length prefix.
pub const ARTIFACT_BLOB_HEADER_LENGTH_BYTES: usize = 4;
/// Maximum stored blob size.
pub const MAX_BLOB_BYTES: u64 = 512 * 1024 * 1024;
