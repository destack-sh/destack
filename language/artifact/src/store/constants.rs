/// File extension used for immutable artifact record segments.
pub const ARTIFACT_SEGMENT_EXTENSION: &str = "segment";
/// Maximum stored blob size.
pub const MAX_BLOB_BYTES: u64 = 512 * 1024 * 1024;
/// Artifact store layout and record format version.
pub(crate) const ARTIFACT_STORE_VERSION: u32 = 7;
