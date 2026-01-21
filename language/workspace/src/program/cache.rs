use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use postcard::Error as PostcardError;
use rustc_hash::FxHasher;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use destack_source::{CACHE_FORMAT_VERSION, CACHE_MAGIC, CacheHeader, CacheKind, ModuleId};

use crate::{CacheStoreError, ModuleAstData, ModuleDirData, ModuleMirData, ProfileId};

// limit cache entry size to avoid excessive memory usage
pub const CACHE_ENTRY_LIMIT_BYTES: u64 = 512 * 1024 * 1024;

/// Cache entry for serialized module AST data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleAstCacheEntry {
    /// The cache header for this entry.
    pub header: CacheHeader,
    /// The cached AST payload.
    pub payload: ModuleAstData,
}

impl ModuleAstCacheEntry {
    /// Create a new AST cache entry.
    pub fn new(mut header: CacheHeader, payload: ModuleAstData) -> Result<Self, CacheError> {
        header.payload_hash = payload_hash_from_payload(&payload)?;
        Ok(Self { header, payload })
    }

    /// Create a new AST cache entry without computing a payload hash.
    pub fn new_unchecked(mut header: CacheHeader, payload: ModuleAstData) -> Self {
        header.payload_hash = 0;
        Self { header, payload }
    }

    /// Validate the cache header for this entry.
    pub fn validate(&self) -> Result<(), CacheError> {
        validate_header(&self.header, CacheKind::Ast)
            .and_then(|_| validate_payload_hash(&self.header, &self.payload))
    }
}

/// Cache entry for serialized module DIR data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDirCacheEntry {
    /// The cache header for this entry.
    pub header: CacheHeader,
    /// The cached DIR payload.
    pub payload: ModuleDirData,
}

impl ModuleDirCacheEntry {
    /// Create a new DIR cache entry.
    pub fn new(mut header: CacheHeader, payload: ModuleDirData) -> Result<Self, CacheError> {
        header.payload_hash = payload_hash_from_payload(&payload)?;
        Ok(Self { header, payload })
    }

    /// Create a new DIR cache entry without computing a payload hash.
    pub fn new_unchecked(mut header: CacheHeader, payload: ModuleDirData) -> Self {
        header.payload_hash = 0;
        Self { header, payload }
    }

    /// Validate the cache header for this entry with a specific kind.
    pub fn validate_for_kind(&self, expected_kind: CacheKind) -> Result<(), CacheError> {
        validate_header(&self.header, expected_kind)
            .and_then(|_| validate_payload_hash(&self.header, &self.payload))
    }

    /// Validate the cache header for this entry.
    pub fn validate(&self) -> Result<(), CacheError> {
        self.validate_for_kind(CacheKind::DirResolved)
    }
}

/// Cache entry for serialized module MIR data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMirCacheEntry {
    /// The cache header for this entry.
    pub header: CacheHeader,
    /// The cached MIR payload.
    pub payload: ModuleMirData,
}

impl ModuleMirCacheEntry {
    /// Create a new MIR cache entry.
    pub fn new(mut header: CacheHeader, payload: ModuleMirData) -> Result<Self, CacheError> {
        header.payload_hash = payload_hash_from_payload(&payload)?;
        Ok(Self { header, payload })
    }

    /// Create a new MIR cache entry without computing a payload hash.
    pub fn new_unchecked(mut header: CacheHeader, payload: ModuleMirData) -> Self {
        header.payload_hash = 0;
        Self { header, payload }
    }

    /// Validate the cache header for this entry.
    pub fn validate(&self) -> Result<(), CacheError> {
        validate_header(&self.header, CacheKind::Mir)
            .and_then(|_| validate_payload_hash(&self.header, &self.payload))
    }
}

/// Errors that can occur while reading or writing cache entries.
#[derive(Debug)]
pub enum CacheError {
    /// The cache header magic did not match.
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },
    /// The cache header format version did not match.
    InvalidFormatVersion { expected: u32, found: u32 },
    /// The cache header kind did not match the expected payload kind.
    InvalidCacheKind {
        expected: CacheKind,
        found: CacheKind,
    },
    /// The cache entry failed to serialize.
    Serialize(PostcardError),
    /// The cache entry failed to deserialize.
    Deserialize(PostcardError),
    /// The cache entry exceeded the configured size limit.
    SizeLimitExceeded { limit: u64, actual: u64 },
    /// The cache entry payload hash did not match.
    InvalidPayloadHash { expected: u64, found: u64 },
    /// The cache entry failed to read or write.
    Io(std::io::Error),
    /// Missing dependency data for cache validation.
    MissingDependencyData {
        /// The module id for the missing dependency data.
        module_id: ModuleId,
        /// The profile id for the missing dependency data.
        profile_id: Option<ProfileId>,
        /// Description of the missing data.
        reason: String,
    },
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // format cache errors
        match self {
            CacheError::InvalidMagic { expected, found } => {
                write!(
                    f,
                    "invalid cache magic, expected {expected:?}, found {found:?}"
                )
            }
            CacheError::InvalidFormatVersion { expected, found } => {
                write!(
                    f,
                    "invalid cache format version, expected {expected}, found {found}"
                )
            }
            CacheError::InvalidCacheKind { expected, found } => {
                write!(
                    f,
                    "invalid cache kind, expected {expected:?}, found {found:?}"
                )
            }
            CacheError::Serialize(error) => write!(f, "failed to serialize cache entry: {error}"),
            CacheError::Deserialize(error) => {
                write!(f, "failed to deserialize cache entry: {error}")
            }
            CacheError::SizeLimitExceeded { limit, actual } => {
                write!(
                    f,
                    "cache entry exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            CacheError::InvalidPayloadHash { expected, found } => {
                write!(
                    f,
                    "invalid payload hash, expected {expected}, found {found}"
                )
            }
            CacheError::Io(error) => write!(f, "cache io error: {error}"),
            CacheError::MissingDependencyData {
                module_id,
                profile_id,
                reason,
            } => {
                let profile_display = profile_id
                    .map(|profile_id| format!("{profile_id:?}"))
                    .unwrap_or_else(|| "none".to_string());
                write!(
                    f,
                    "missing dependency data for {module_id:?} at {profile_display}: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for CacheError {}

impl From<CacheStoreError> for CacheError {
    fn from(error: CacheStoreError) -> Self {
        match error {
            CacheStoreError::Io(error) => CacheError::Io(error),
        }
    }
}

/// Validate a cache header against the expected kind and format.
pub fn validate_header(header: &CacheHeader, expected_kind: CacheKind) -> Result<(), CacheError> {
    // validate cache magic
    if header.magic != CACHE_MAGIC {
        return Err(CacheError::InvalidMagic {
            expected: CACHE_MAGIC,
            found: header.magic,
        });
    }

    // validate cache format version
    if header.format_version != CACHE_FORMAT_VERSION {
        return Err(CacheError::InvalidFormatVersion {
            expected: CACHE_FORMAT_VERSION,
            found: header.format_version,
        });
    }

    // validate cache kind
    if header.cache_kind != expected_kind {
        return Err(CacheError::InvalidCacheKind {
            expected: expected_kind,
            found: header.cache_kind,
        });
    }

    Ok(())
}

/// Compute the payload hash for serialized bytes.
pub fn payload_hash_from_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Compute a payload hash for a cache payload.
pub fn payload_hash_from_payload<T: Serialize>(payload: &T) -> Result<u64, CacheError> {
    let bytes = serialize_cache_entry_with_limit(payload, CACHE_ENTRY_LIMIT_BYTES)?;
    Ok(payload_hash_from_bytes(&bytes))
}

/// Serialize a cache entry with a size limit.
fn serialize_cache_entry_with_limit<T: Serialize>(
    entry: &T,
    limit: u64,
) -> Result<Vec<u8>, CacheError> {
    // serialize with bounded size
    let bytes = postcard::to_allocvec(entry).map_err(CacheError::Serialize)?;

    // enforce cache entry size limit
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(CacheError::SizeLimitExceeded { limit, actual });
    }

    Ok(bytes)
}

/// Deserialize a cache entry with a size limit.
fn deserialize_cache_entry_with_limit<T: DeserializeOwned>(
    bytes: &[u8],
    limit: u64,
) -> Result<T, CacheError> {
    // enforce cache entry size limit
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(CacheError::SizeLimitExceeded { limit, actual });
    }

    // deserialize cache entry
    let entry = postcard::from_bytes(bytes).map_err(CacheError::Deserialize)?;

    Ok(entry)
}

/// Validate a payload hash against a cache header.
fn validate_payload_hash<T: Serialize>(
    header: &CacheHeader,
    payload: &T,
) -> Result<(), CacheError> {
    // compute payload hash
    let payload_hash = payload_hash_from_payload(payload)?;

    // compare against header
    if payload_hash != header.payload_hash {
        return Err(CacheError::InvalidPayloadHash {
            expected: header.payload_hash,
            found: payload_hash,
        });
    }

    Ok(())
}

/// Serialize a cache entry to bytes.
pub fn serialize_cache_entry<T: Serialize>(entry: &T) -> Result<Vec<u8>, CacheError> {
    serialize_cache_entry_with_limit(entry, CACHE_ENTRY_LIMIT_BYTES)
}

/// Deserialize a cache entry from bytes.
pub fn deserialize_cache_entry<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CacheError> {
    deserialize_cache_entry_with_limit(bytes, CACHE_ENTRY_LIMIT_BYTES)
}

/// Write a cache entry to a path.
pub fn write_cache_entry<T: Serialize>(path: &Path, entry: &T) -> Result<(), CacheError> {
    // serialize cache entry
    let bytes = serialize_cache_entry(entry)?;

    // write cache entry
    write_cache_bytes_atomic(path, &bytes)?;

    Ok(())
}

/// Read a cache entry from a path.
pub fn read_cache_entry<T: DeserializeOwned>(path: &Path) -> Result<T, CacheError> {
    // read cache bytes
    let bytes = std::fs::read(path).map_err(CacheError::Io)?;

    // deserialize cache entry
    let entry = deserialize_cache_entry(&bytes)?;

    Ok(entry)
}

/// Write cache bytes using an atomic replace.
fn write_cache_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), CacheError> {
    // resolve target directory and file name
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cache");

    // build a temp path for atomic replace
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_name = format!("{file_name}.tmp-{}-{timestamp}", std::process::id());
    let temp_path = dir.join(temp_name);

    // write and sync temp file
    let mut file = std::fs::File::create(&temp_path).map_err(CacheError::Io)?;
    file.write_all(bytes).map_err(CacheError::Io)?;
    file.sync_all().map_err(CacheError::Io)?;

    // best effort directory sync
    if let Ok(dir_file) = std::fs::File::open(dir) {
        let _ = dir_file.sync_all();
    }

    // rename into place with fallback for existing targets
    if let Err(error) = std::fs::rename(&temp_path, path) {
        if path.exists() {
            let _ = std::fs::remove_file(path);
            std::fs::rename(&temp_path, path).map_err(CacheError::Io)?;
        } else {
            return Err(CacheError::Io(error));
        }
    }

    Ok(())
}

/// Read and validate a module AST cache entry from a path.
pub fn read_module_ast_cache(path: &Path) -> Result<ModuleAstCacheEntry, CacheError> {
    // read cache entry
    let entry: ModuleAstCacheEntry = read_cache_entry(path)?;

    // validate cache entry
    entry.validate()?;

    Ok(entry)
}

/// Write a module AST cache entry to a path after validation.
pub fn write_module_ast_cache(path: &Path, entry: &ModuleAstCacheEntry) -> Result<(), CacheError> {
    // validate cache entry
    entry.validate()?;

    // write cache entry
    write_cache_entry(path, entry)?;

    Ok(())
}

/// Read and validate a module DIR cache entry from a path.
pub fn read_module_dir_base_cache(path: &Path) -> Result<ModuleDirCacheEntry, CacheError> {
    // read cache entry
    let entry: ModuleDirCacheEntry = read_cache_entry(path)?;

    // validate cache entry
    entry.validate_for_kind(CacheKind::DirBase)?;

    Ok(entry)
}

/// Write a base DIR cache entry to a path after validation.
pub fn write_module_dir_base_cache(
    path: &Path,
    entry: &ModuleDirCacheEntry,
) -> Result<(), CacheError> {
    // validate cache entry
    entry.validate_for_kind(CacheKind::DirBase)?;

    // write cache entry
    write_cache_entry(path, entry)?;

    Ok(())
}

/// Read and validate a resolved DIR cache entry from a path.
pub fn read_module_dir_resolved_cache(path: &Path) -> Result<ModuleDirCacheEntry, CacheError> {
    // read cache entry
    let entry: ModuleDirCacheEntry = read_cache_entry(path)?;

    // validate cache entry
    entry.validate_for_kind(CacheKind::DirResolved)?;

    Ok(entry)
}

/// Write a resolved DIR cache entry to a path after validation.
pub fn write_module_dir_resolved_cache(
    path: &Path,
    entry: &ModuleDirCacheEntry,
) -> Result<(), CacheError> {
    // validate cache entry
    entry.validate_for_kind(CacheKind::DirResolved)?;

    // write cache entry
    write_cache_entry(path, entry)?;

    Ok(())
}

/// Read and validate an analyzed DIR cache entry from a path.
pub fn read_module_dir_analyzed_cache(path: &Path) -> Result<ModuleDirCacheEntry, CacheError> {
    // read cache entry
    let entry: ModuleDirCacheEntry = read_cache_entry(path)?;

    // validate cache entry
    entry.validate_for_kind(CacheKind::DirAnalyzed)?;

    Ok(entry)
}

/// Write an analyzed DIR cache entry to a path after validation.
pub fn write_module_dir_analyzed_cache(
    path: &Path,
    entry: &ModuleDirCacheEntry,
) -> Result<(), CacheError> {
    // validate cache entry
    entry.validate_for_kind(CacheKind::DirAnalyzed)?;

    // write cache entry
    write_cache_entry(path, entry)?;

    Ok(())
}

/// Read and validate an executed DIR cache entry from a path.
pub fn read_module_dir_executed_cache(path: &Path) -> Result<ModuleDirCacheEntry, CacheError> {
    // read cache entry
    let entry: ModuleDirCacheEntry = read_cache_entry(path)?;

    // validate cache entry
    entry.validate_for_kind(CacheKind::DirExecuted)?;

    Ok(entry)
}

/// Write an executed DIR cache entry to a path after validation.
pub fn write_module_dir_executed_cache(
    path: &Path,
    entry: &ModuleDirCacheEntry,
) -> Result<(), CacheError> {
    // validate cache entry
    entry.validate_for_kind(CacheKind::DirExecuted)?;

    // write cache entry
    write_cache_entry(path, entry)?;

    Ok(())
}

/// Read and validate a module MIR cache entry from a path.
pub fn read_module_mir_cache(path: &Path) -> Result<ModuleMirCacheEntry, CacheError> {
    // read cache entry
    let entry: ModuleMirCacheEntry = read_cache_entry(path)?;

    // validate cache entry
    entry.validate()?;

    Ok(entry)
}

/// Write a module MIR cache entry to a path after validation.
pub fn write_module_mir_cache(path: &Path, entry: &ModuleMirCacheEntry) -> Result<(), CacheError> {
    // validate cache entry
    entry.validate()?;

    // write cache entry
    write_cache_entry(path, entry)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use destack_source::{
        FileId, FileVersion, ModuleId, ModuleVersion, PackageId, ProfileId, ProfileVersion,
        TemporaryPhysicalFileSystem,
    };

    use crate::{ModuleAst, ModuleDir, ModuleMir, TargetId};

    use super::*;

    /// Build a cache header for tests.
    fn test_header(kind: CacheKind) -> CacheHeader {
        let profile_id = if kind.requires_profile() {
            Some(ProfileId::new(1))
        } else {
            None
        };
        let profile_version = if kind.requires_profile() {
            Some(ProfileVersion::new(1))
        } else {
            None
        };
        CacheHeader::new(
            kind,
            "test".to_string(),
            ModuleId::EPHEMERAL,
            FileVersion::INITIAL,
            profile_id,
            profile_version,
            0,
            0,
            0,
            0,
            0,
        )
    }

    fn test_anchor_source_id() -> u32 {
        // create a minimal anchor for dir cache entries
        let mut ast = ModuleAst::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL);
        let anchor_id = ast.ensure_anchor_expression(FileId::new(0));
        anchor_id.id
    }

    /// Build a cache file path under a temporary root.
    fn cache_path(root: &TemporaryPhysicalFileSystem, name: &str) -> PathBuf {
        let filename = format!("{name}.bin");
        root.path_for(&filename)
    }

    #[test]
    fn test_module_ast_cache_roundtrip() {
        // roundtrip module ast cache entries through disk
        let module_ast = ModuleAst::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL);
        let entry =
            ModuleAstCacheEntry::new(test_header(CacheKind::Ast), module_ast.to_data()).unwrap();
        let root = TemporaryPhysicalFileSystem::new_with_prefix("module_ast_cache");
        let path = cache_path(&root, "ast");

        write_module_ast_cache(&path, &entry).expect("write module ast cache");
        let loaded = read_module_ast_cache(&path).expect("read module ast cache");

        // assert essential header and payload fields
        assert_eq!(loaded.header.cache_kind, CacheKind::Ast);
        assert_eq!(loaded.payload.id, ModuleId::EPHEMERAL);
        assert_eq!(loaded.payload.version, ModuleVersion::INITIAL);
        assert_eq!(loaded.payload.roots.len(), 0);
    }

    #[test]
    fn test_module_dir_base_cache_roundtrip() {
        // roundtrip module dir base cache entries through disk
        let anchor_source_id = test_anchor_source_id();
        let module_dir = ModuleDir::new_base(
            ModuleId::EPHEMERAL,
            ModuleVersion::INITIAL,
            anchor_source_id,
        );
        let entry = ModuleDirCacheEntry::new(test_header(CacheKind::DirBase), module_dir.to_data())
            .unwrap();
        let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_base_cache");
        let path = cache_path(&root, "dir-base");

        write_module_dir_base_cache(&path, &entry).expect("write module dir base cache");
        let loaded = read_module_dir_base_cache(&path).expect("read module dir base cache");

        // assert essential header and payload fields
        assert_eq!(loaded.header.cache_kind, CacheKind::DirBase);
        assert_eq!(loaded.payload.id, ModuleId::EPHEMERAL);
        assert_eq!(loaded.payload.version, ModuleVersion::INITIAL);
        assert!(loaded.payload.profile_id.is_none());
    }

    #[test]
    fn test_module_dir_resolved_cache_roundtrip() {
        // roundtrip module dir resolved cache entries through disk
        let anchor_source_id = test_anchor_source_id();
        let base = ModuleDir::new_base(
            ModuleId::EPHEMERAL,
            ModuleVersion::INITIAL,
            anchor_source_id,
        );
        let module_dir = ModuleDir::from_base(&base, ProfileId::new(1));
        let entry =
            ModuleDirCacheEntry::new(test_header(CacheKind::DirResolved), module_dir.to_data())
                .unwrap();
        let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_resolved_cache");
        let path = cache_path(&root, "dir-resolved");

        write_module_dir_resolved_cache(&path, &entry).expect("write module dir resolved cache");
        let loaded = read_module_dir_resolved_cache(&path).expect("read module dir resolved cache");

        // assert essential header and payload fields
        assert_eq!(loaded.header.cache_kind, CacheKind::DirResolved);
        assert_eq!(loaded.payload.id, ModuleId::EPHEMERAL);
        assert_eq!(loaded.payload.version, ModuleVersion::INITIAL);
        assert_eq!(loaded.payload.profile_id, Some(ProfileId::new(1)));
    }

    #[test]
    fn test_module_dir_analyzed_cache_roundtrip() {
        // roundtrip analyzed dir cache entries through disk
        let anchor_source_id = test_anchor_source_id();
        let base = ModuleDir::new_base(
            ModuleId::EPHEMERAL,
            ModuleVersion::INITIAL,
            anchor_source_id,
        );
        let module_dir = ModuleDir::from_base(&base, ProfileId::new(1));
        let entry =
            ModuleDirCacheEntry::new(test_header(CacheKind::DirAnalyzed), module_dir.to_data())
                .unwrap();
        let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_analyzed_cache");
        let path = cache_path(&root, "dir-analyzed");

        write_module_dir_analyzed_cache(&path, &entry).expect("write analyzed dir cache");
        let loaded = read_module_dir_analyzed_cache(&path).expect("read analyzed dir cache");

        // assert essential header and payload fields
        assert_eq!(loaded.header.cache_kind, CacheKind::DirAnalyzed);
        assert_eq!(loaded.payload.id, ModuleId::EPHEMERAL);
        assert_eq!(loaded.payload.version, ModuleVersion::INITIAL);
        assert_eq!(loaded.payload.profile_id, Some(ProfileId::new(1)));
    }

    #[test]
    fn test_module_dir_executed_cache_roundtrip() {
        // roundtrip executed dir cache entries through disk
        let anchor_source_id = test_anchor_source_id();
        let base = ModuleDir::new_base(
            ModuleId::EPHEMERAL,
            ModuleVersion::INITIAL,
            anchor_source_id,
        );
        let module_dir = ModuleDir::from_base(&base, ProfileId::new(1));
        let entry =
            ModuleDirCacheEntry::new(test_header(CacheKind::DirExecuted), module_dir.to_data())
                .unwrap();
        let root = TemporaryPhysicalFileSystem::new_with_prefix("dir_executed_cache");
        let path = cache_path(&root, "dir-executed");

        write_module_dir_executed_cache(&path, &entry).expect("write executed dir cache");
        let loaded = read_module_dir_executed_cache(&path).expect("read executed dir cache");

        // assert essential header and payload fields
        assert_eq!(loaded.header.cache_kind, CacheKind::DirExecuted);
        assert_eq!(loaded.payload.id, ModuleId::EPHEMERAL);
        assert_eq!(loaded.payload.version, ModuleVersion::INITIAL);
        assert_eq!(loaded.payload.profile_id, Some(ProfileId::new(1)));
    }

    #[test]
    fn test_module_mir_cache_roundtrip() {
        // roundtrip module mir cache entries through disk
        let target = TargetId::new(PackageId::EPHEMERAL, "test");
        let module_mir = ModuleMir::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL, target);
        let entry =
            ModuleMirCacheEntry::new(test_header(CacheKind::Mir), module_mir.to_data()).unwrap();
        let root = TemporaryPhysicalFileSystem::new_with_prefix("mir_cache");
        let path = cache_path(&root, "mir");

        write_module_mir_cache(&path, &entry).expect("write module mir cache");
        let loaded = read_module_mir_cache(&path).expect("read module mir cache");

        // assert essential header and payload fields
        assert_eq!(loaded.header.cache_kind, CacheKind::Mir);
        assert_eq!(loaded.payload.id, ModuleId::EPHEMERAL);
        assert_eq!(loaded.payload.version, ModuleVersion::INITIAL);
    }

    #[test]
    fn test_cache_invalid_magic() {
        // reject invalid cache magic
        let module_ast = ModuleAst::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL);
        let mut entry =
            ModuleAstCacheEntry::new(test_header(CacheKind::Ast), module_ast.to_data()).unwrap();
        entry.header.magic = [0; 4];

        let result = entry.validate();
        assert!(matches!(result, Err(CacheError::InvalidMagic { .. })));
    }

    #[test]
    fn test_cache_invalid_kind() {
        // reject wrong cache kind
        let module_ast = ModuleAst::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL);
        let mut entry =
            ModuleAstCacheEntry::new(test_header(CacheKind::Ast), module_ast.to_data()).unwrap();
        entry.header.cache_kind = CacheKind::DirResolved;

        let result = entry.validate();
        assert!(matches!(result, Err(CacheError::InvalidCacheKind { .. })));
    }

    #[test]
    fn test_cache_payload_hash_mismatch() {
        // reject payload hash mismatch
        let module_ast = ModuleAst::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL);
        let mut entry =
            ModuleAstCacheEntry::new(test_header(CacheKind::Ast), module_ast.to_data()).unwrap();
        entry.header.payload_hash = entry.header.payload_hash.wrapping_add(1);

        let result = entry.validate();
        assert!(matches!(result, Err(CacheError::InvalidPayloadHash { .. })));
    }

    #[test]
    fn test_cache_truncated_bytes() {
        // reject truncated cache entry
        let module_ast = ModuleAst::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL);
        let entry =
            ModuleAstCacheEntry::new(test_header(CacheKind::Ast), module_ast.to_data()).unwrap();
        let bytes = serialize_cache_entry(&entry).expect("serialize cache entry");
        let truncated = &bytes[..bytes.len() / 2];
        let result: Result<ModuleAstCacheEntry, CacheError> = deserialize_cache_entry(truncated);
        assert!(matches!(result, Err(CacheError::Deserialize(_))));
    }

    #[test]
    fn test_cache_size_limit() {
        // enforce size limits on cache serialization
        let module_ast = ModuleAst::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL);
        let entry =
            ModuleAstCacheEntry::new(test_header(CacheKind::Ast), module_ast.to_data()).unwrap();
        let result = serialize_cache_entry_with_limit(&entry, 1);
        assert!(matches!(result, Err(CacheError::SizeLimitExceeded { .. })));
    }
}
