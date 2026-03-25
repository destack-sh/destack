/// Magic prefix for on disk artifact images.
pub const ARTIFACT_IMAGE_MAGIC: [u8; 4] = *b"DSCH";
/// Artifact image format version.
pub const ARTIFACT_IMAGE_FORMAT_VERSION: u32 = 6;
/// Limit artifact image size to avoid excessive memory usage.
pub const ARTIFACT_IMAGE_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
/// Number of bytes used to encode one header length prefix.
pub const ARTIFACT_IMAGE_HEADER_LENGTH_BYTES: usize = 4;
