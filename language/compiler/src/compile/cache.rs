use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use filetime::{FileTime, set_file_mtime};
use fs2::FileExt;
use rustc_hash::FxHasher;
use serde_json::Value;

use destack_source::{
    CacheHeader, CacheKind, FileContent, FileId, FileVersion, ModuleId, ProfileId, ProfileVersion,
    TargetId,
};
use destack_workspace::{
    CACHE_ENTRY_LIMIT_BYTES, CacheError, CacheMode, CachePolicy, CacheScope, CacheValidate,
    ModuleAstCacheEntry, ModuleAstData, ModuleDirCacheEntry, ModuleDirData, ModuleMirCacheEntry,
    ModuleMirData, read_cache_entry, resolve_global_cache_root, serialize_cache_entry,
    write_cache_entry,
};

use super::{Compiler, CompilerStats};

/// Default cache directory name for workspace scoped caches.
pub const DEFAULT_CACHE_DIR: &str = ".destack";
/// Default cache directory name for global caches.
pub const DEFAULT_GLOBAL_CACHE_DIR: &str = "destack";
/// Namespace for compiler cache entries.
pub const DEFAULT_CACHE_NAMESPACE: &str = "compiler";
/// Cache lock file name.
const CACHE_LOCK_FILE: &str = "cache.lock";
/// Bytes per megabyte.
const BYTES_PER_MB: u64 = 1024 * 1024;
const DSCONFIG_CACHE_IGNORED_KEYS: [&str; 6] = [
    "cache",
    "watch",
    "formatter",
    "linter",
    "targets",
    "defaultTarget",
];

/// Cache settings for a module.
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// Cache mode.
    pub mode: CacheMode,
    /// Cache directory path.
    pub dir: PathBuf,
    /// Cache eviction policy.
    pub policy: CachePolicy,
    /// Cache validation strategy.
    pub validate: CacheValidate,
    /// Cache scope selection.
    pub scope: CacheScope,
    /// Maximum cache size in megabytes.
    pub max_size_mb: Option<u64>,
}

impl CacheSettings {
    /// Check whether cache is enabled for this module.
    pub fn is_enabled(&self) -> bool {
        self.mode != CacheMode::Off
    }

    /// Check whether disk caching is enabled for this module.
    pub fn is_disk_enabled(&self) -> bool {
        self.mode == CacheMode::Disk
    }
}

/// Cache context for a module.
#[derive(Debug, Clone)]
pub struct CacheContext {
    /// The compiler version that produced the cache.
    pub compiler_version: String,
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
}

impl CacheContext {
    /// Build a cache header for this context.
    pub fn header(&self, cache_kind: CacheKind, module_id: ModuleId) -> CacheHeader {
        CacheHeader::new(
            cache_kind,
            self.compiler_version.clone(),
            module_id,
            self.file_version,
            self.profile_id,
            self.profile_version,
            self.source_hash,
            self.config_hash,
            self.target_hash,
            0,
        )
    }
}

/// Cache key for in memory entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// The cache kind.
    pub cache_kind: CacheKind,
    /// The module id.
    pub module_id: ModuleId,
    /// The file version.
    pub file_version: FileVersion,
    /// The profile id.
    pub profile_id: ProfileId,
    /// The profile version.
    pub profile_version: ProfileVersion,
    /// The source hash.
    pub source_hash: u64,
    /// The config hash.
    pub config_hash: u64,
    /// The target hash.
    pub target_hash: u64,
}

impl CacheKey {
    /// Create a cache key from a context.
    pub fn new(cache_kind: CacheKind, module_id: ModuleId, context: &CacheContext) -> Self {
        Self {
            cache_kind,
            module_id,
            file_version: context.file_version,
            profile_id: context.profile_id,
            profile_version: context.profile_version,
            source_hash: context.source_hash,
            config_hash: context.config_hash,
            target_hash: context.target_hash,
        }
    }
}

/// Cached entry stored in memory with access metadata.
#[derive(Debug, Clone)]
struct CacheMemoryEntry<T> {
    /// The cached entry payload.
    entry: T,
    /// Serialized size of the entry in bytes.
    size_bytes: u64,
    /// When the entry was created.
    created_at: Instant,
    /// When the entry was last accessed.
    last_access: Instant,
}

impl<T> CacheMemoryEntry<T> {
    /// Create a new memory cache entry.
    fn new(entry: T, size_bytes: u64) -> Self {
        let now = Instant::now();
        Self {
            entry,
            size_bytes,
            created_at: now,
            last_access: now,
        }
    }

    /// Update the last access timestamp.
    fn touch(&mut self) {
        self.last_access = Instant::now();
    }

    /// Get the eviction key for the configured policy.
    fn eviction_key(&self, policy: CachePolicy) -> Instant {
        match policy {
            CachePolicy::Lru => self.last_access,
            CachePolicy::Ttl => self.created_at,
        }
    }
}

/// Disk cache size tracking state.
#[derive(Debug)]
struct CacheDiskState {
    /// Cached disk size in bytes.
    size_bytes: AtomicU64,
    /// Whether the disk size has been initialized.
    initialized: AtomicBool,
}

impl CacheDiskState {
    /// Create a new disk state tracker.
    fn new() -> Self {
        Self {
            size_bytes: AtomicU64::new(0),
            initialized: AtomicBool::new(false),
        }
    }
}

/// Memory cache size tracking state.
#[derive(Debug)]
struct CacheMemoryState {
    /// Cached memory size in bytes.
    size_bytes: AtomicU64,
}

impl CacheMemoryState {
    /// Create a new memory state tracker.
    fn new() -> Self {
        Self {
            size_bytes: AtomicU64::new(0),
        }
    }
}

/// In memory cache registry for compiler artifacts.
#[derive(Debug)]
pub struct CacheRegistry {
    /// Cached AST entries.
    ast: DashMap<CacheKey, CacheMemoryEntry<ModuleAstCacheEntry>>,
    /// Cached DIR entries.
    dir: DashMap<CacheKey, CacheMemoryEntry<ModuleDirCacheEntry>>,
    /// Cached MIR entries.
    mir: DashMap<CacheKey, CacheMemoryEntry<ModuleMirCacheEntry>>,
    /// Memory cache tracking state.
    memory_state: CacheMemoryState,
    /// Disk cache tracking state.
    disk_state: CacheDiskState,
}

/// Cache file entry used for eviction.
#[derive(Debug)]
struct CacheFileEntry {
    /// The file path on disk.
    path: PathBuf,
    /// The file size in bytes.
    size: u64,
    /// The last accessed timestamp when available.
    accessed: Option<SystemTime>,
    /// The last modified timestamp when available.
    modified: Option<SystemTime>,
}

/// Identify the cache map for in-memory eviction.
#[derive(Debug, Clone, Copy)]
enum MemoryCacheKind {
    /// AST cache entry.
    Ast,
    /// DIR cache entry.
    Dir,
    /// MIR cache entry.
    Mir,
}

/// Memory cache entry used for eviction.
#[derive(Debug)]
struct MemoryEvictionEntry {
    /// Cache map for the entry.
    kind: MemoryCacheKind,
    /// Cache key for the entry.
    key: CacheKey,
    /// Size of the cached entry in bytes.
    size_bytes: u64,
    /// Eviction key for ordering.
    eviction_key: Instant,
}

impl CacheFileEntry {
    /// Return an eviction key for the configured policy.
    fn eviction_key(&self, policy: CachePolicy) -> u128 {
        // pick the time source for eviction
        let preferred = match policy {
            CachePolicy::Lru => self.modified.or(self.accessed),
            CachePolicy::Ttl => self.modified.or(self.accessed),
        };

        // convert to a sortable key
        let timestamp = preferred.unwrap_or(UNIX_EPOCH);
        timestamp
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    }
}

/// Cache read source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheReadSource {
    /// Cache entry was loaded from memory.
    Memory,
    /// Cache entry was loaded from disk.
    Disk,
}

/// Cache read outcome.
#[derive(Debug)]
enum CacheReadOutcome<T> {
    /// Cache entry was found.
    Hit { entry: T, source: CacheReadSource },
    /// Cache entry was not found.
    Miss,
    /// Cache entry failed to load.
    Error,
}

/// Cache handle bound to module, profile, and target context.
#[derive(Debug, Clone)]
pub struct CacheHandle<'a> {
    /// The cache registry.
    registry: &'a CacheRegistry,
    /// The cache stats.
    stats: &'a CompilerStats,
    /// The cache settings.
    settings: CacheSettings,
    /// The cache context.
    context: CacheContext,
    /// The module id.
    module_id: ModuleId,
}

impl CacheHandle<'_> {
    /// Read an AST cache entry if available.
    pub fn read_ast(&self) -> Result<Option<ModuleAstCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !self.settings.is_enabled() {
            return Ok(None);
        }

        // read from cache registry
        let outcome =
            self.registry
                .read_ast_cache_outcome(&self.settings, &self.context, self.module_id);

        // record cache outcome
        match &outcome {
            Ok(CacheReadOutcome::Hit { source, .. }) => {
                self.record_cache_hit(*source, CacheKind::Ast);
            }
            Ok(CacheReadOutcome::Miss) => {
                self.stats.record_cache_ast_miss();
            }
            Ok(CacheReadOutcome::Error) | Err(_) => {
                self.stats.record_cache_error();
            }
        }

        // map cache outcome to response
        match outcome {
            Ok(CacheReadOutcome::Hit { entry, .. }) => Ok(Some(entry)),
            Ok(CacheReadOutcome::Miss) | Ok(CacheReadOutcome::Error) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// Write an AST cache entry.
    pub fn write_ast(&self, payload: ModuleAstData) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !self.settings.is_enabled() {
            return Ok(());
        }

        // write to cache registry
        let result =
            self.registry
                .write_ast_cache(&self.settings, &self.context, self.module_id, payload);

        // record cache outcome
        match &result {
            Ok(()) => {
                self.stats.record_cache_ast_write_memory();
                if self.settings.is_disk_enabled() {
                    self.stats.record_cache_ast_write_disk();
                }
            }
            Err(_) => {
                self.stats.record_cache_error();
            }
        }

        result
    }

    /// Read a DIR cache entry if available.
    pub fn read_dir(&self) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !self.settings.is_enabled() {
            return Ok(None);
        }

        // read from cache registry
        let outcome =
            self.registry
                .read_dir_cache_outcome(&self.settings, &self.context, self.module_id);

        // record cache outcome
        match &outcome {
            Ok(CacheReadOutcome::Hit { source, .. }) => {
                self.record_cache_hit(*source, CacheKind::Dir);
            }
            Ok(CacheReadOutcome::Miss) => {
                self.stats.record_cache_dir_miss();
            }
            Ok(CacheReadOutcome::Error) | Err(_) => {
                self.stats.record_cache_error();
            }
        }

        // map cache outcome to response
        match outcome {
            Ok(CacheReadOutcome::Hit { entry, .. }) => Ok(Some(entry)),
            Ok(CacheReadOutcome::Miss) | Ok(CacheReadOutcome::Error) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// Write a DIR cache entry.
    pub fn write_dir(&self, payload: ModuleDirData) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !self.settings.is_enabled() {
            return Ok(());
        }

        // write to cache registry
        let result =
            self.registry
                .write_dir_cache(&self.settings, &self.context, self.module_id, payload);

        // record cache outcome
        match &result {
            Ok(()) => {
                self.stats.record_cache_dir_write_memory();
                if self.settings.is_disk_enabled() {
                    self.stats.record_cache_dir_write_disk();
                }
            }
            Err(_) => {
                self.stats.record_cache_error();
            }
        }

        result
    }

    /// Read a MIR cache entry if available.
    pub fn read_mir(&self) -> Result<Option<ModuleMirCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !self.settings.is_enabled() {
            return Ok(None);
        }

        // read from cache registry
        let outcome =
            self.registry
                .read_mir_cache_outcome(&self.settings, &self.context, self.module_id);

        // record cache outcome
        match &outcome {
            Ok(CacheReadOutcome::Hit { source, .. }) => {
                self.record_cache_hit(*source, CacheKind::Mir);
            }
            Ok(CacheReadOutcome::Miss) => {
                self.stats.record_cache_mir_miss();
            }
            Ok(CacheReadOutcome::Error) | Err(_) => {
                self.stats.record_cache_error();
            }
        }

        // map cache outcome to response
        match outcome {
            Ok(CacheReadOutcome::Hit { entry, .. }) => Ok(Some(entry)),
            Ok(CacheReadOutcome::Miss) | Ok(CacheReadOutcome::Error) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// Write a MIR cache entry.
    pub fn write_mir(&self, payload: ModuleMirData) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !self.settings.is_enabled() {
            return Ok(());
        }

        // write to cache registry
        let result =
            self.registry
                .write_mir_cache(&self.settings, &self.context, self.module_id, payload);

        // record cache outcome
        match &result {
            Ok(()) => {
                self.stats.record_cache_mir_write_memory();
                if self.settings.is_disk_enabled() {
                    self.stats.record_cache_mir_write_disk();
                }
            }
            Err(_) => {
                self.stats.record_cache_error();
            }
        }

        result
    }

    /// Record a cache hit by source for a cache kind.
    fn record_cache_hit(&self, source: CacheReadSource, cache_kind: CacheKind) {
        // record cache hit by kind
        match cache_kind {
            CacheKind::Ast => match source {
                CacheReadSource::Memory => self.stats.record_cache_ast_hit_memory(),
                CacheReadSource::Disk => self.stats.record_cache_ast_hit_disk(),
            },
            CacheKind::Dir => match source {
                CacheReadSource::Memory => self.stats.record_cache_dir_hit_memory(),
                CacheReadSource::Disk => self.stats.record_cache_dir_hit_disk(),
            },
            CacheKind::Mir => match source {
                CacheReadSource::Memory => self.stats.record_cache_mir_hit_memory(),
                CacheReadSource::Disk => self.stats.record_cache_mir_hit_disk(),
            },
        }
    }
}

impl CacheRegistry {
    /// Create a new cache registry.
    pub fn new() -> Self {
        Self {
            ast: DashMap::new(),
            dir: DashMap::new(),
            mir: DashMap::new(),
            memory_state: CacheMemoryState::new(),
            disk_state: CacheDiskState::new(),
        }
    }

    /// Read an AST cache entry if available.
    pub fn read_ast_cache(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<ModuleAstCacheEntry>, CacheError> {
        // read entry outcome from cache
        let outcome = self.read_ast_cache_outcome(settings, context, module_id)?;

        // map cache outcome to response
        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write an AST cache entry.
    pub fn write_ast_cache(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
        payload: ModuleAstData,
    ) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !settings.is_enabled() {
            return Ok(());
        }

        // build the entry and store in memory
        let header = context.header(CacheKind::Ast, module_id);
        let entry = if settings.is_disk_enabled() {
            ModuleAstCacheEntry::new(header, payload)?
        } else {
            ModuleAstCacheEntry::new_unchecked(header, payload)
        };
        let key = CacheKey::new(CacheKind::Ast, module_id, context);
        let size_bytes = self.entry_size_bytes(settings, &entry)?;
        self.insert_memory_entry(
            &self.ast,
            key.clone(),
            CacheMemoryEntry::new(entry.clone(), size_bytes),
        );
        self.evict_memory_entries(settings)?;

        // stop when disk cache is disabled
        if !settings.is_disk_enabled() {
            return Ok(());
        }

        // write to disk
        self.write_cache_entry(settings, &key, &entry)
    }

    /// Read a DIR cache entry if available.
    pub fn read_dir_cache(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        // read entry outcome from cache
        let outcome = self.read_dir_cache_outcome(settings, context, module_id)?;

        // map cache outcome to response
        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write a DIR cache entry.
    pub fn write_dir_cache(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
        payload: ModuleDirData,
    ) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !settings.is_enabled() {
            return Ok(());
        }

        // build the entry and store in memory
        let header = context.header(CacheKind::Dir, module_id);
        let entry = if settings.is_disk_enabled() {
            ModuleDirCacheEntry::new(header, payload)?
        } else {
            ModuleDirCacheEntry::new_unchecked(header, payload)
        };
        let key = CacheKey::new(CacheKind::Dir, module_id, context);
        let size_bytes = self.entry_size_bytes(settings, &entry)?;
        self.insert_memory_entry(
            &self.dir,
            key.clone(),
            CacheMemoryEntry::new(entry.clone(), size_bytes),
        );
        self.evict_memory_entries(settings)?;

        // stop when disk cache is disabled
        if !settings.is_disk_enabled() {
            return Ok(());
        }

        // write to disk
        self.write_cache_entry(settings, &key, &entry)
    }

    /// Read a MIR cache entry if available.
    pub fn read_mir_cache(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<ModuleMirCacheEntry>, CacheError> {
        // read entry outcome from cache
        let outcome = self.read_mir_cache_outcome(settings, context, module_id)?;

        // map cache outcome to response
        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Read an AST cache entry with its source.
    fn read_ast_cache_outcome(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<ModuleAstCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !settings.is_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try in memory cache first
        let key = CacheKey::new(CacheKind::Ast, module_id, context);
        if let Some(mut entry_ref) = self.ast.get_mut(&key) {
            let entry = entry_ref.entry.clone();
            match self.validate_ast_entry(&entry, context, settings) {
                Ok(true) => {
                    entry_ref.touch();
                    return Ok(CacheReadOutcome::Hit {
                        entry,
                        source: CacheReadSource::Memory,
                    });
                }
                Ok(false) => {
                    drop(entry_ref);
                    self.remove_memory_entry(&self.ast, &key);
                }
                Err(_) => {
                    drop(entry_ref);
                    self.remove_memory_entry(&self.ast, &key);
                    return Ok(CacheReadOutcome::Error);
                }
            }
        }

        // stop when disk cache is disabled
        if !settings.is_disk_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try disk cache
        let path = self.cache_entry_path(settings, &key);
        if !path.exists() {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: ModuleAstCacheEntry = match self.read_disk_entry(settings, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(settings, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        };
        match self.validate_ast_entry(&entry, context, settings) {
            Ok(true) => {
                if let Ok(size_bytes) = self.entry_size_bytes(settings, &entry) {
                    self.insert_memory_entry(
                        &self.ast,
                        key,
                        CacheMemoryEntry::new(entry.clone(), size_bytes),
                    );
                    self.evict_memory_entries(settings)?;
                }
                self.touch_disk_entry(settings, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(settings, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(settings, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Read a DIR cache entry with its source.
    fn read_dir_cache_outcome(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<ModuleDirCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !settings.is_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try in memory cache first
        let key = CacheKey::new(CacheKind::Dir, module_id, context);
        if let Some(mut entry_ref) = self.dir.get_mut(&key) {
            let entry = entry_ref.entry.clone();
            match self.validate_dir_entry(&entry, context, settings) {
                Ok(true) => {
                    entry_ref.touch();
                    return Ok(CacheReadOutcome::Hit {
                        entry,
                        source: CacheReadSource::Memory,
                    });
                }
                Ok(false) => {
                    drop(entry_ref);
                    self.remove_memory_entry(&self.dir, &key);
                }
                Err(_) => {
                    drop(entry_ref);
                    self.remove_memory_entry(&self.dir, &key);
                    return Ok(CacheReadOutcome::Error);
                }
            }
        }

        // stop when disk cache is disabled
        if !settings.is_disk_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try disk cache
        let path = self.cache_entry_path(settings, &key);
        if !path.exists() {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: ModuleDirCacheEntry = match self.read_disk_entry(settings, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(settings, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        };
        match self.validate_dir_entry(&entry, context, settings) {
            Ok(true) => {
                if let Ok(size_bytes) = self.entry_size_bytes(settings, &entry) {
                    self.insert_memory_entry(
                        &self.dir,
                        key,
                        CacheMemoryEntry::new(entry.clone(), size_bytes),
                    );
                    self.evict_memory_entries(settings)?;
                }
                self.touch_disk_entry(settings, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(settings, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(settings, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Read a MIR cache entry with its source.
    fn read_mir_cache_outcome(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<ModuleMirCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !settings.is_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try in memory cache first
        let key = CacheKey::new(CacheKind::Mir, module_id, context);
        if let Some(mut entry_ref) = self.mir.get_mut(&key) {
            let entry = entry_ref.entry.clone();
            match self.validate_mir_entry(&entry, context, settings) {
                Ok(true) => {
                    entry_ref.touch();
                    return Ok(CacheReadOutcome::Hit {
                        entry,
                        source: CacheReadSource::Memory,
                    });
                }
                Ok(false) => {
                    drop(entry_ref);
                    self.remove_memory_entry(&self.mir, &key);
                }
                Err(_) => {
                    drop(entry_ref);
                    self.remove_memory_entry(&self.mir, &key);
                    return Ok(CacheReadOutcome::Error);
                }
            }
        }

        // stop when disk cache is disabled
        if !settings.is_disk_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try disk cache
        let path = self.cache_entry_path(settings, &key);
        if !path.exists() {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: ModuleMirCacheEntry = match self.read_disk_entry(settings, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(settings, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        };
        match self.validate_mir_entry(&entry, context, settings) {
            Ok(true) => {
                if let Ok(size_bytes) = self.entry_size_bytes(settings, &entry) {
                    self.insert_memory_entry(
                        &self.mir,
                        key,
                        CacheMemoryEntry::new(entry.clone(), size_bytes),
                    );
                    self.evict_memory_entries(settings)?;
                }
                self.touch_disk_entry(settings, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(settings, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(settings, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Write a MIR cache entry.
    pub fn write_mir_cache(
        &self,
        settings: &CacheSettings,
        context: &CacheContext,
        module_id: ModuleId,
        payload: ModuleMirData,
    ) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !settings.is_enabled() {
            return Ok(());
        }

        // build the entry and store in memory
        let header = context.header(CacheKind::Mir, module_id);
        let entry = if settings.is_disk_enabled() {
            ModuleMirCacheEntry::new(header, payload)?
        } else {
            ModuleMirCacheEntry::new_unchecked(header, payload)
        };
        let key = CacheKey::new(CacheKind::Mir, module_id, context);
        let size_bytes = self.entry_size_bytes(settings, &entry)?;
        self.insert_memory_entry(
            &self.mir,
            key.clone(),
            CacheMemoryEntry::new(entry.clone(), size_bytes),
        );
        self.evict_memory_entries(settings)?;

        // stop when disk cache is disabled
        if !settings.is_disk_enabled() {
            return Ok(());
        }

        // write to disk
        self.write_cache_entry(settings, &key, &entry)
    }

    /// Insert a cache entry into a memory map and track size.
    fn insert_memory_entry<T>(
        &self,
        map: &DashMap<CacheKey, CacheMemoryEntry<T>>,
        key: CacheKey,
        entry: CacheMemoryEntry<T>,
    ) {
        let size_bytes = entry.size_bytes;
        if let Some(existing) = map.insert(key, entry) {
            self.decrement_memory_size(existing.size_bytes);
        }
        self.increment_memory_size(size_bytes);
    }

    /// Remove a cache entry from a memory map and track size.
    fn remove_memory_entry<T>(
        &self,
        map: &DashMap<CacheKey, CacheMemoryEntry<T>>,
        key: &CacheKey,
    ) -> Option<CacheMemoryEntry<T>> {
        map.remove(key).map(|(_, entry)| {
            self.decrement_memory_size(entry.size_bytes);
            entry
        })
    }

    /// Compute the serialized entry size in bytes when limits are enabled.
    fn entry_size_bytes<T: serde::Serialize>(
        &self,
        settings: &CacheSettings,
        entry: &T,
    ) -> Result<u64, CacheError> {
        let Some(max_size_mb) = settings.max_size_mb else {
            return Ok(0);
        };
        if max_size_mb == 0 {
            return Ok(0);
        }

        let bytes = serialize_cache_entry(entry)?;
        Ok(bytes.len() as u64)
    }

    /// Evict in memory cache entries when size limits are exceeded.
    fn evict_memory_entries(&self, settings: &CacheSettings) -> Result<(), CacheError> {
        let Some(max_size_mb) = settings.max_size_mb else {
            return Ok(());
        };

        if max_size_mb == 0 {
            return Ok(());
        }

        let limit_bytes = max_size_mb.saturating_mul(BYTES_PER_MB);
        let mut total_size = self.memory_state.size_bytes.load(Ordering::Relaxed);
        if total_size <= limit_bytes {
            return Ok(());
        }

        let mut entries = Vec::new();
        self.collect_memory_entries(settings.policy, &mut entries);
        entries.sort_by_key(|entry| entry.eviction_key);

        for entry in entries {
            if total_size <= limit_bytes {
                break;
            }

            let removed = match entry.kind {
                MemoryCacheKind::Ast => self.remove_memory_entry(&self.ast, &entry.key).is_some(),
                MemoryCacheKind::Dir => self.remove_memory_entry(&self.dir, &entry.key).is_some(),
                MemoryCacheKind::Mir => self.remove_memory_entry(&self.mir, &entry.key).is_some(),
            };

            if removed {
                total_size = total_size.saturating_sub(entry.size_bytes);
            }
        }

        Ok(())
    }

    /// Collect all memory cache entries for eviction.
    fn collect_memory_entries(&self, policy: CachePolicy, entries: &mut Vec<MemoryEvictionEntry>) {
        for entry in self.ast.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::Ast,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }

        for entry in self.dir.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::Dir,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }

        for entry in self.mir.iter() {
            entries.push(MemoryEvictionEntry {
                kind: MemoryCacheKind::Mir,
                key: entry.key().clone(),
                size_bytes: entry.value().size_bytes,
                eviction_key: entry.value().eviction_key(policy),
            });
        }
    }

    /// Record memory size additions.
    fn increment_memory_size(&self, size_bytes: u64) {
        if size_bytes == 0 {
            return;
        }
        self.memory_state
            .size_bytes
            .fetch_add(size_bytes, Ordering::Relaxed);
    }

    /// Record memory size removals.
    fn decrement_memory_size(&self, size_bytes: u64) {
        if size_bytes == 0 {
            return;
        }

        let mut current = self.memory_state.size_bytes.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_sub(size_bytes);
            match self.memory_state.size_bytes.compare_exchange(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(updated) => current = updated,
            }
        }
    }

    /// Read a cache entry from disk with size limits and locking.
    fn read_disk_entry<T: serde::de::DeserializeOwned>(
        &self,
        settings: &CacheSettings,
        path: &Path,
    ) -> Result<T, CacheError> {
        let _lock = self.lock_cache_shared(settings)?;
        self.check_disk_entry_size(path)?;
        read_cache_entry(path)
    }

    /// Remove a cache entry from disk and update size tracking.
    fn remove_disk_entry(&self, settings: &CacheSettings, path: &Path) -> Result<(), CacheError> {
        let _lock = self.lock_cache_exclusive(settings)?;
        let size = if settings.max_size_mb.is_some() {
            std::fs::metadata(path)
                .map(|metadata| metadata.len())
                .unwrap_or_default()
        } else {
            0
        };
        if let Err(error) = std::fs::remove_file(path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(CacheError::Io(error));
            }
        }

        if size > 0 {
            self.decrement_disk_size(size);
        }
        Ok(())
    }

    /// Ensure the disk cache size is tracked before eviction.
    fn ensure_disk_size_initialized(&self, settings: &CacheSettings) -> Result<(), CacheError> {
        if self.disk_state.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }

        let root = settings.dir.join(DEFAULT_CACHE_NAMESPACE);
        if !root.exists() {
            self.disk_state.size_bytes.store(0, Ordering::Relaxed);
            self.disk_state.initialized.store(true, Ordering::Relaxed);
            return Ok(());
        }

        let mut entries = Vec::new();
        self.collect_cache_entries(&root, &mut entries)?;
        let total_size = entries.iter().map(|entry| entry.size).sum::<u64>();
        self.disk_state
            .size_bytes
            .store(total_size, Ordering::Relaxed);
        self.disk_state.initialized.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Update the disk cache size after writes.
    fn increment_disk_size(&self, size_bytes: u64) {
        if size_bytes == 0 {
            return;
        }
        self.disk_state
            .size_bytes
            .fetch_add(size_bytes, Ordering::Relaxed);
    }

    /// Update the disk cache size after removals.
    fn decrement_disk_size(&self, size_bytes: u64) {
        if size_bytes == 0 {
            return;
        }

        let mut current = self.disk_state.size_bytes.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_sub(size_bytes);
            match self.disk_state.size_bytes.compare_exchange(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(updated) => current = updated,
            }
        }
    }

    /// Check whether a cache entry exceeds the size limit.
    fn check_disk_entry_size(&self, path: &Path) -> Result<(), CacheError> {
        let metadata = std::fs::metadata(path).map_err(CacheError::Io)?;
        let size = metadata.len();
        if size > CACHE_ENTRY_LIMIT_BYTES {
            return Err(CacheError::SizeLimitExceeded {
                limit: CACHE_ENTRY_LIMIT_BYTES,
                actual: size,
            });
        }

        Ok(())
    }

    /// Touch a disk cache entry for LRU policies.
    fn touch_disk_entry(&self, settings: &CacheSettings, path: &Path) {
        if settings.policy != CachePolicy::Lru {
            return;
        }

        let now = FileTime::from_system_time(SystemTime::now());
        let _ = set_file_mtime(path, now);
    }

    /// Acquire a shared disk cache lock.
    fn lock_cache_shared(
        &self,
        settings: &CacheSettings,
    ) -> Result<Option<std::fs::File>, CacheError> {
        let root = settings.dir.join(DEFAULT_CACHE_NAMESPACE);
        if !root.exists() {
            return Ok(None);
        }
        let path = root.join(CACHE_LOCK_FILE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(CacheError::Io)?;
        file.lock_shared().map_err(CacheError::Io)?;
        Ok(Some(file))
    }

    /// Acquire an exclusive disk cache lock.
    fn lock_cache_exclusive(
        &self,
        settings: &CacheSettings,
    ) -> Result<Option<std::fs::File>, CacheError> {
        let root = settings.dir.join(DEFAULT_CACHE_NAMESPACE);
        std::fs::create_dir_all(&root).map_err(CacheError::Io)?;
        let path = root.join(CACHE_LOCK_FILE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(CacheError::Io)?;
        file.lock_exclusive().map_err(CacheError::Io)?;
        Ok(Some(file))
    }

    /// Validate an AST cache entry against the context.
    fn validate_ast_entry(
        &self,
        entry: &ModuleAstCacheEntry,
        context: &CacheContext,
        settings: &CacheSettings,
    ) -> Result<bool, CacheError> {
        // compare headers first
        let expected = context.header(CacheKind::Ast, entry.header.module_id);
        if !self.header_matches(&expected, &entry.header) {
            return Ok(false);
        }

        // validate payload when strict
        if settings.is_disk_enabled() && settings.validate == CacheValidate::Strict {
            entry.validate()?;
        }

        Ok(true)
    }

    /// Validate a DIR cache entry against the context.
    fn validate_dir_entry(
        &self,
        entry: &ModuleDirCacheEntry,
        context: &CacheContext,
        settings: &CacheSettings,
    ) -> Result<bool, CacheError> {
        // compare headers first
        let expected = context.header(CacheKind::Dir, entry.header.module_id);
        if !self.header_matches(&expected, &entry.header) {
            return Ok(false);
        }

        // validate payload when strict
        if settings.is_disk_enabled() && settings.validate == CacheValidate::Strict {
            entry.validate()?;
        }

        Ok(true)
    }

    /// Validate a MIR cache entry against the context.
    fn validate_mir_entry(
        &self,
        entry: &ModuleMirCacheEntry,
        context: &CacheContext,
        settings: &CacheSettings,
    ) -> Result<bool, CacheError> {
        // compare headers first
        let expected = context.header(CacheKind::Mir, entry.header.module_id);
        if !self.header_matches(&expected, &entry.header) {
            return Ok(false);
        }

        // validate payload when strict
        if settings.is_disk_enabled() && settings.validate == CacheValidate::Strict {
            entry.validate()?;
        }

        Ok(true)
    }

    /// Compare two cache headers for equality.
    fn header_matches(&self, expected: &CacheHeader, actual: &CacheHeader) -> bool {
        // compare cache header fields
        expected.magic == actual.magic
            && expected.format_version == actual.format_version
            && expected.compiler_version == actual.compiler_version
            && expected.cache_kind == actual.cache_kind
            && expected.module_id == actual.module_id
            && expected.file_version == actual.file_version
            && expected.profile_id == actual.profile_id
            && expected.profile_version == actual.profile_version
            && expected.source_hash == actual.source_hash
            && expected.config_hash == actual.config_hash
            && expected.target_hash == actual.target_hash
    }

    /// Write a cache entry to disk after ensuring its directory exists.
    fn write_cache_entry<T: serde::Serialize>(
        &self,
        settings: &CacheSettings,
        key: &CacheKey,
        entry: &T,
    ) -> Result<(), CacheError> {
        let _lock = self.lock_cache_exclusive(settings)?;

        // ensure cache directory exists
        let path = self.cache_entry_path_from_root(&settings.dir, key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(CacheError::Io)?;
        }

        // capture size before write
        let previous_size = std::fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or_default();

        let track_disk_size = settings.max_size_mb.is_some();
        if track_disk_size {
            self.ensure_disk_size_initialized(settings)?;
        }

        // write cache entry to disk
        write_cache_entry(&path, entry)?;

        // update disk cache size tracking
        let new_size = std::fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or_default();
        if track_disk_size {
            if new_size > previous_size {
                self.increment_disk_size(new_size - previous_size);
            } else if previous_size > new_size {
                self.decrement_disk_size(previous_size - new_size);
            }
        }

        // enforce cache size limits when configured
        self.evict_cache_entries(settings)?;

        Ok(())
    }

    /// Build the cache entry path for a key.
    fn cache_entry_path(&self, settings: &CacheSettings, key: &CacheKey) -> PathBuf {
        self.cache_entry_path_from_root(&settings.dir, key)
    }

    /// Build the cache entry path for a root directory.
    fn cache_entry_path_from_root(&self, root: &Path, key: &CacheKey) -> PathBuf {
        // pick the cache kind directory
        let kind_dir = match key.cache_kind {
            CacheKind::Ast => "ast",
            CacheKind::Dir => "dir",
            CacheKind::Mir => "mir",
        };

        // format cache key segments
        let package = format!("{:016x}", key.module_id.package_id.raw());
        let module = format!("{:08x}", key.module_id.local_id);
        let file_version = key.file_version.0;
        let profile_id = key.profile_id.raw();
        let profile_version = key.profile_version.0;
        let file_name = format!(
            "{kind_dir}-f{file_version}-p{profile_id}-pv{profile_version}-s{source_hash:016x}-c{config_hash:016x}-t{target_hash:016x}.bin",
            source_hash = key.source_hash,
            config_hash = key.config_hash,
            target_hash = key.target_hash,
        );

        // build the cache path
        root.join(DEFAULT_CACHE_NAMESPACE)
            .join(kind_dir)
            .join(package)
            .join(module)
            .join(file_name)
    }

    /// Evict cache entries when size limits are exceeded.
    fn evict_cache_entries(&self, settings: &CacheSettings) -> Result<(), CacheError> {
        // skip when no size limit is configured
        let Some(max_size_mb) = settings.max_size_mb else {
            return Ok(());
        };

        // skip when the configured limit is zero
        if max_size_mb == 0 {
            return Ok(());
        }

        // compute the size limit in bytes
        let limit_bytes = max_size_mb.saturating_mul(BYTES_PER_MB);
        self.ensure_disk_size_initialized(settings)?;
        let mut total_size = self.disk_state.size_bytes.load(Ordering::Relaxed);
        if total_size <= limit_bytes {
            return Ok(());
        }

        // resolve the cache root directory
        let root = settings.dir.join(DEFAULT_CACHE_NAMESPACE);
        if !root.exists() {
            return Ok(());
        }

        // collect cache entries for eviction
        let mut entries = Vec::new();
        self.collect_cache_entries(&root, &mut entries)?;

        // sum cache size across entries
        // sort entries by eviction key
        entries.sort_by_key(|entry| entry.eviction_key(settings.policy));

        // evict oldest entries until under limit
        for entry in entries {
            if total_size <= limit_bytes {
                break;
            }

            // remove cache entry from disk
            if let Err(error) = std::fs::remove_file(&entry.path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    return Err(CacheError::Io(error));
                }
            }

            total_size = total_size.saturating_sub(entry.size);
            self.decrement_disk_size(entry.size);
        }

        Ok(())
    }

    /// Collect cache entries for eviction.
    fn collect_cache_entries(
        &self,
        root: &Path,
        entries: &mut Vec<CacheFileEntry>,
    ) -> Result<(), CacheError> {
        // read directory entries
        let read_dir = std::fs::read_dir(root).map_err(CacheError::Io)?;
        for entry in read_dir {
            let entry = entry.map_err(CacheError::Io)?;
            let path = entry.path();
            let metadata = entry.metadata().map_err(CacheError::Io)?;

            // collect nested entries for directories
            if metadata.is_dir() {
                self.collect_cache_entries(&path, entries)?;
                continue;
            }

            // skip non file entries
            if !metadata.is_file() {
                continue;
            }

            // skip lock files
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name == CACHE_LOCK_FILE)
            {
                continue;
            }

            // collect cache file entry metadata
            let accessed = metadata.accessed().ok();
            let modified = metadata.modified().ok();
            entries.push(CacheFileEntry {
                path,
                size: metadata.len(),
                accessed,
                modified,
            });
        }

        Ok(())
    }
}

impl Default for CacheRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    /// Resolve cache settings for a module.
    pub(crate) fn cache_settings_for_module(&self, module_id: ModuleId) -> CacheSettings {
        // load module and package
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package = self.program.packages.get(module.package_id);
        let package = package.read();

        // derive cache options from dsconfig
        let cache_options = package
            .dsconfig
            .as_ref()
            .map(|dsconfig| dsconfig.options.cache.clone())
            .unwrap_or_default();

        // resolve cache root directory
        let workspace_root = self.session.workspace.root.clone();
        let mut dir = if let Some(dsconfig) = package.dsconfig.as_ref()
            && let Some(cache_dir) = cache_options.dir.as_ref()
        {
            if cache_dir.is_absolute() {
                cache_dir.clone()
            } else {
                dsconfig.directory.join(cache_dir)
            }
        } else {
            workspace_root.join(DEFAULT_CACHE_DIR)
        };

        // apply global cache scope
        if cache_options.scope == CacheScope::Global
            && let Some(global_root) = resolve_global_cache_root(DEFAULT_GLOBAL_CACHE_DIR)
        {
            dir = global_root;
        }

        // build resolved cache settings
        CacheSettings {
            mode: cache_options.mode,
            dir,
            policy: cache_options.policy,
            validate: cache_options.validate,
            scope: cache_options.scope,
            max_size_mb: cache_options.max_size_mb,
        }
    }

    /// Build a cache context for a module.
    pub(crate) fn cache_context_for_module(
        &self,
        module_id: ModuleId,
        profile_id: Option<ProfileId>,
        target_id: Option<&TargetId>,
    ) -> Result<CacheContext, CacheError> {
        // load module file id
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let file_id = module.file_id;

        // derive profile info
        let (resolved_profile_id, profile_version) = if let Some(profile_id) = profile_id {
            let profile = self.program.profile(profile_id);
            (profile_id, profile.version)
        } else {
            (ProfileId::new(0), ProfileVersion::INITIAL)
        };

        // compute file and config hashes
        let file_version = self.cache_file_version(file_id)?;
        let source_hash = self.cache_source_hash(file_id)?;
        let config_hash = self.cache_config_hash(module_id, profile_id);
        let target_hash = self.cache_target_hash(module_id, target_id);

        // build cache context
        Ok(CacheContext {
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            file_version,
            profile_id: resolved_profile_id,
            profile_version,
            source_hash,
            config_hash,
            target_hash,
        })
    }

    /// Build cache handle for a module when available.
    pub(crate) fn cache_handle_for_module(
        &self,
        module_id: ModuleId,
        profile_id: Option<ProfileId>,
        target_id: Option<&TargetId>,
    ) -> Option<CacheHandle<'_>> {
        // resolve cache settings
        let settings = self.cache_settings_for_module(module_id);

        // skip when cache is disabled
        if !settings.is_enabled() {
            return None;
        }

        // resolve cache context
        let context = self
            .cache_context_for_module(module_id, profile_id, target_id)
            .ok()?;

        Some(CacheHandle {
            registry: &self.cache,
            stats: self.stats.as_ref(),
            settings,
            context,
            module_id,
        })
    }

    /// Compute the file version for a cache context.
    fn cache_file_version(&self, file_id: FileId) -> Result<FileVersion, CacheError> {
        let file = self.program.files.get(file_id);
        Ok(file.version)
    }

    /// Compute the source hash for a cache context.
    fn cache_source_hash(&self, file_id: FileId) -> Result<u64, CacheError> {
        // hash file content
        let file = self.program.files.get(file_id);
        match &file.content {
            FileContent::Text { content } => Ok(hash_bytes(content.as_bytes())),
            FileContent::Json { content, .. } => Ok(hash_bytes(content.as_bytes())),
            FileContent::Binary { content } => Ok(hash_bytes(content)),
            FileContent::Unloaded => Err(CacheError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file content not loaded",
            ))),
        }
    }

    /// Compute the config hash for a cache context.
    fn cache_config_hash(&self, module_id: ModuleId, profile_id: Option<ProfileId>) -> u64 {
        // seed the config hash
        let mut hasher = FxHasher::default();

        // hash the dsconfig content if available
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package = self.program.packages.get(module.package_id);
        let package = package.read();
        if let Some(dsconfig) = package.dsconfig.as_ref() {
            let file = self.program.files.get(dsconfig.file_id);
            match &file.content {
                FileContent::Json { value, .. } => {
                    hash_dsconfig_value(value).hash(&mut hasher);
                }
                FileContent::Text { content } => {
                    hash_bytes(content.as_bytes()).hash(&mut hasher);
                }
                FileContent::Binary { content } => {
                    hash_bytes(content).hash(&mut hasher);
                }
                FileContent::Unloaded => {}
            }
        }

        // hash the profile key if provided
        if let Some(profile_id) = profile_id {
            let profile = self.program.profile(profile_id);
            profile.key.hash(&mut hasher);
        }

        // finish the config hash
        hasher.finish()
    }

    /// Compute the target hash for a cache context.
    fn cache_target_hash(&self, module_id: ModuleId, target_id: Option<&TargetId>) -> u64 {
        // hash the target id when present
        let mut hasher = FxHasher::default();
        if let Some(target_id) = target_id {
            target_id.hash(&mut hasher);

            // hash the resolved target config when available
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            if let Some(target) = package.targets.get(target_id) {
                target.hash(&mut hasher);
            }
        }

        hasher.finish()
    }
}

/// Hash bytes with a stable hasher.
fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Hash a JSON object with a stable key ordering.
fn hash_json_object(
    map: &serde_json::Map<String, Value>,
    hasher: &mut FxHasher,
    filter_keys: Option<&[&str]>,
) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(filter) = filter_keys {
            if filter.iter().any(|ignored| ignored == &key.as_str()) {
                continue;
            }
        }

        key.hash(hasher);
        if let Some(value) = map.get(key) {
            hash_json_value(value, hasher);
        }
    }
}

/// Hash a JSON value with canonical ordering for object keys.
fn hash_json_value(value: &Value, hasher: &mut FxHasher) {
    match value {
        Value::Null => {
            0_u8.hash(hasher);
        }
        Value::Bool(value) => {
            1_u8.hash(hasher);
            value.hash(hasher);
        }
        Value::Number(value) => {
            2_u8.hash(hasher);
            value.to_string().hash(hasher);
        }
        Value::String(value) => {
            3_u8.hash(hasher);
            value.hash(hasher);
        }
        Value::Array(values) => {
            4_u8.hash(hasher);
            values.len().hash(hasher);
            for entry in values {
                hash_json_value(entry, hasher);
            }
        }
        Value::Object(map) => {
            5_u8.hash(hasher);
            hash_json_object(map, hasher, None);
        }
    }
}

/// Hash a dsconfig JSON value for cache purposes.
fn hash_dsconfig_value(value: &Value) -> u64 {
    let mut hasher = FxHasher::default();
    match value {
        Value::Object(map) => {
            5_u8.hash(&mut hasher);
            hash_json_object(map, &mut hasher, Some(&DSCONFIG_CACHE_IGNORED_KEYS));
        }
        _ => hash_json_value(value, &mut hasher),
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::hash_dsconfig_value;
    use serde_json::json;

    /// Hashing ignores dsconfig key order for cache invalidation.
    #[test]
    fn test_dsconfig_hash_is_order_invariant() {
        let first = json!({
            "compilerOptions": { "strict": true, "noImplicitAny": true },
            "include": ["src"],
            "exclude": ["dist"],
        });
        let second = json!({
            "exclude": ["dist"],
            "include": ["src"],
            "compilerOptions": { "noImplicitAny": true, "strict": true },
        });

        // assert hashes are stable across key order
        assert_eq!(hash_dsconfig_value(&first), hash_dsconfig_value(&second));
    }

    /// Hashing changes when relevant config values change.
    #[test]
    fn test_dsconfig_hash_changes_on_value_change() {
        let first = json!({
            "compilerOptions": { "strict": true },
            "include": ["src"],
        });
        let second = json!({
            "compilerOptions": { "strict": false },
            "include": ["src"],
        });

        // assert hashes diverge for semantic changes
        assert_ne!(hash_dsconfig_value(&first), hash_dsconfig_value(&second));
    }

    /// Hashing ignores tooling-only sections that do not affect compilation.
    #[test]
    fn test_dsconfig_hash_ignores_tooling_sections() {
        let first = json!({
            "compilerOptions": { "strict": true },
            "cache": { "mode": "disk" },
            "watch": { "debounceMs": 10 },
            "formatter": { "lineWidth": 100 },
            "linter": { "preset": "recommended" },
        });
        let second = json!({
            "compilerOptions": { "strict": true },
            "cache": { "mode": "memory" },
            "watch": { "debounceMs": 50 },
            "formatter": { "lineWidth": 80 },
            "linter": { "preset": "strict" },
        });

        // assert hashes match despite tooling-only changes
        assert_eq!(hash_dsconfig_value(&first), hash_dsconfig_value(&second));
    }
}
