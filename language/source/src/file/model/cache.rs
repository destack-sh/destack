use serde::{Deserialize, Serialize};

use crate::{FileVersion, ModuleId, ProfileId, ProfileVersion};

/// Magic prefix for on disk cache headers.
pub const CACHE_MAGIC: [u8; 4] = *b"DSCH";
/// Cache header format version.
pub const CACHE_FORMAT_VERSION: u32 = 1;

/// Kind of cached payload stored after the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheKind {
    /// AST cache payload.
    Ast,
    /// DIR cache payload.
    Dir,
    /// MIR cache payload.
    Mir,
}

/// Common cache header for on disk compiler artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheHeader {
    /// The magic prefix used to identify cache files.
    pub magic: [u8; 4],
    /// The cache header format version.
    pub format_version: u32,
    /// The compiler version that produced the cache.
    pub compiler_version: String,
    /// The payload kind stored after this header.
    pub cache_kind: CacheKind,
    /// The module id for the cached payload.
    pub module_id: ModuleId,
    /// The file version used when producing the cache.
    pub file_version: FileVersion,
    /// The profile id for the cached payload.
    pub profile_id: ProfileId,
    /// The profile version used when producing the cache.
    pub profile_version: ProfileVersion,
    /// Hash of the normalized source contents.
    pub source_hash: u64,
    /// Hash of the effective compiler configuration.
    pub config_hash: u64,
    /// Hash of the target configuration and triple.
    pub target_hash: u64,
    /// Hash of the serialized payload bytes.
    pub payload_hash: u64,
}

#[allow(clippy::too_many_arguments)]
impl CacheHeader {
    /// Create a new CacheHeader.
    pub fn new(
        cache_kind: CacheKind,
        compiler_version: String,
        module_id: ModuleId,
        file_version: FileVersion,
        profile_id: ProfileId,
        profile_version: ProfileVersion,
        source_hash: u64,
        config_hash: u64,
        target_hash: u64,
        payload_hash: u64,
    ) -> Self {
        // populate header fields
        Self {
            magic: CACHE_MAGIC,
            format_version: CACHE_FORMAT_VERSION,
            compiler_version,
            cache_kind,
            module_id,
            file_version,
            profile_id,
            profile_version,
            source_hash,
            config_hash,
            target_hash,
            payload_hash,
        }
    }
}
