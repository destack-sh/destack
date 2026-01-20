use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use destack_source::{CacheHeader, CacheKind};
use destack_workspace::{
    CACHE_ENTRY_LIMIT_BYTES, CacheError, CacheLock, CachePolicy, CacheStore, CacheStoreKind,
    CacheValidate, ModuleAstCacheEntry, ModuleDirCacheEntry, ModuleMirCacheEntry,
    deserialize_cache_entry, serialize_cache_entry,
};

use super::context::BYTES_PER_MB;
use super::{
    CacheContext, CacheKey, CacheOptions, CacheRegistry, DEFAULT_COMPILER_CACHE_NAMESPACE,
};

const CACHE_LOCK_FILE: &str = "cache.lock";
const CACHE_ACCESS_SUFFIX: &str = ".access";

/// Disk cache size tracking state.
#[derive(Debug)]
pub(super) struct CacheDiskState {
    /// Cached disk size in bytes.
    pub(super) size_bytes: AtomicU64,
    /// Whether the disk size has been initialized.
    pub(super) initialized: AtomicBool,
}

impl CacheDiskState {
    /// Create a new disk state tracker.
    pub(super) fn new() -> Self {
        Self {
            size_bytes: AtomicU64::new(0),
            initialized: AtomicBool::new(false),
        }
    }
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

impl CacheFileEntry {
    /// Return an eviction key for the configured policy.
    fn eviction_key(&self, policy: CachePolicy) -> u128 {
        // pick the time source for eviction
        let preferred = match policy {
            CachePolicy::Lru => self.accessed.or(self.modified),
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

impl CacheRegistry {
    /// Read a cache entry from disk with size limits and locking.
    pub(super) fn read_disk_entry<T: serde::de::DeserializeOwned>(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        path: &Path,
    ) -> Result<T, CacheError> {
        // acquire shared cache lock
        let _lock = self.lock_cache_shared(cache_store, options)?;

        // preflight entry size when metadata is available
        if let Some(metadata) = cache_store.metadata(path).map_err(CacheError::from)?
            && metadata.size_bytes > CACHE_ENTRY_LIMIT_BYTES
        {
            return Err(CacheError::SizeLimitExceeded {
                limit: CACHE_ENTRY_LIMIT_BYTES,
                actual: metadata.size_bytes,
            });
        }

        // read cache bytes
        let Some(bytes) = cache_store.read(path).map_err(CacheError::from)? else {
            return Err(CacheError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "cache entry missing",
            )));
        };

        // guard against oversized entries
        let size = bytes.len() as u64;
        if size > CACHE_ENTRY_LIMIT_BYTES {
            return Err(CacheError::SizeLimitExceeded {
                limit: CACHE_ENTRY_LIMIT_BYTES,
                actual: size,
            });
        }

        // decode cached entry
        deserialize_cache_entry(&bytes)
    }

    /// Remove a cache entry from disk and update size tracking.
    pub(super) fn remove_disk_entry(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        path: &Path,
    ) -> Result<(), CacheError> {
        let _lock = self.lock_cache_exclusive(cache_store, options)?;
        let size = if options.max_size_mb.is_some() {
            cache_store
                .metadata(path)
                .map_err(CacheError::from)?
                .map(|metadata| metadata.size_bytes)
                .unwrap_or_default()
        } else {
            0
        };
        cache_store.remove(path).map_err(CacheError::from)?;
        let access_path = self.cache_access_path(path);
        let _ = cache_store.remove(&access_path);

        if size > 0 {
            self.decrement_disk_size(size);
        }
        Ok(())
    }

    /// Ensure the disk cache size is tracked before eviction.
    fn ensure_disk_size_initialized(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
    ) -> Result<(), CacheError> {
        if self.disk_state.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }

        if cache_store.kind() != CacheStoreKind::Disk {
            self.disk_state.size_bytes.store(0, Ordering::Relaxed);
            self.disk_state.initialized.store(true, Ordering::Relaxed);
            return Ok(());
        }

        let root = options.dir.join(DEFAULT_COMPILER_CACHE_NAMESPACE);
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

    /// Touch a disk cache entry for LRU policies.
    pub(super) fn touch_disk_entry(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        path: &Path,
    ) {
        if options.policy != CachePolicy::Lru {
            return;
        }
        let access_path = self.cache_access_path(path);
        let access_exists = cache_store.exists(&access_path).unwrap_or(false);
        if access_exists {
            let _ = cache_store.touch(&access_path);
        } else {
            let _ = cache_store.write_atomic(&access_path, &[]);
        }
    }

    /// Acquire a shared disk cache lock.
    fn lock_cache_shared<'a>(
        &self,
        cache_store: &'a dyn CacheStore,
        options: &CacheOptions,
    ) -> Result<CacheLock<'a>, CacheError> {
        let path = self.cache_lock_path(options);
        cache_store.lock_shared(&path).map_err(CacheError::from)
    }

    /// Acquire an exclusive disk cache lock.
    fn lock_cache_exclusive<'a>(
        &self,
        cache_store: &'a dyn CacheStore,
        options: &CacheOptions,
    ) -> Result<CacheLock<'a>, CacheError> {
        let path = self.cache_lock_path(options);
        cache_store.lock_exclusive(&path).map_err(CacheError::from)
    }

    /// Validate an AST cache entry against the context.
    pub(super) fn validate_ast_entry(
        &self,
        entry: &ModuleAstCacheEntry,
        context: &CacheContext,
        options: &CacheOptions,
    ) -> Result<bool, CacheError> {
        // compare headers first
        let expected = context.header(CacheKind::Ast, entry.header.module_id);
        if !self.header_matches(&expected, &entry.header) {
            return Ok(false);
        }

        // validate payload when strict
        if options.is_disk_enabled() && options.validate == CacheValidate::Strict {
            entry.validate()?;
        }

        Ok(true)
    }

    /// Validate a DIR cache entry against the context for a specific stage.
    pub(super) fn validate_dir_entry(
        &self,
        entry: &ModuleDirCacheEntry,
        context: &CacheContext,
        options: &CacheOptions,
        cache_kind: CacheKind,
    ) -> Result<bool, CacheError> {
        // compare headers first
        let expected = context.header(cache_kind, entry.header.module_id);
        if !self.header_matches(&expected, &entry.header) {
            return Ok(false);
        }

        // validate payload when strict
        if options.is_disk_enabled() && options.validate == CacheValidate::Strict {
            entry.validate()?;
        }

        Ok(true)
    }

    /// Validate a MIR cache entry against the context.
    pub(super) fn validate_mir_entry(
        &self,
        entry: &ModuleMirCacheEntry,
        context: &CacheContext,
        options: &CacheOptions,
    ) -> Result<bool, CacheError> {
        // compare headers first
        let expected = context.header(CacheKind::Mir, entry.header.module_id);
        if !self.header_matches(&expected, &entry.header) {
            return Ok(false);
        }

        // validate payload when strict
        if options.is_disk_enabled() && options.validate == CacheValidate::Strict {
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
            && expected.dependency_hash == actual.dependency_hash
    }

    /// Write a cache entry to disk after ensuring its directory exists.
    pub(super) fn write_cache_entry<T: serde::Serialize>(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        key: &CacheKey,
        entry: &T,
    ) -> Result<(), CacheError> {
        let _lock = self.lock_cache_exclusive(cache_store, options)?;

        // ensure cache directory exists
        let path = self.cache_entry_path_from_root(&options.dir, key);
        let previous_size = cache_store
            .metadata(&path)
            .map_err(CacheError::from)?
            .map(|metadata| metadata.size_bytes)
            .unwrap_or_default();

        let track_disk_size = options.max_size_mb.is_some();
        if track_disk_size {
            self.ensure_disk_size_initialized(cache_store, options)?;
        }

        // write cache entry to disk
        let bytes = serialize_cache_entry(entry)?;
        cache_store
            .write_atomic(&path, &bytes)
            .map_err(CacheError::from)?;

        // update disk cache size tracking
        let new_size = bytes.len() as u64;
        if track_disk_size {
            if new_size > previous_size {
                self.increment_disk_size(new_size - previous_size);
            } else if previous_size > new_size {
                self.decrement_disk_size(previous_size - new_size);
            }
        }

        // enforce cache size limits when configured
        self.evict_cache_entries(cache_store, options)?;

        Ok(())
    }

    /// Build the cache entry path for a key.
    pub(super) fn cache_entry_path(&self, options: &CacheOptions, key: &CacheKey) -> PathBuf {
        self.cache_entry_path_from_root(&options.dir, key)
    }

    /// Build the cache lock path for the configured root.
    fn cache_lock_path(&self, options: &CacheOptions) -> PathBuf {
        options
            .dir
            .join(DEFAULT_COMPILER_CACHE_NAMESPACE)
            .join(CACHE_LOCK_FILE)
    }

    /// Build the cache entry path for a root directory.
    fn cache_entry_path_from_root(&self, root: &Path, key: &CacheKey) -> PathBuf {
        // pick the cache kind directory
        let kind_dir = match key.cache_kind {
            CacheKind::Ast => "ast",
            CacheKind::DirBase => "dir-base",
            CacheKind::DirResolved => "dir-resolved",
            CacheKind::DirAnalyzed => "dir-analyzed",
            CacheKind::DirExecuted => "dir-executed",
            CacheKind::Mir => "mir",
        };

        // format cache key segments
        let package = format!("{:016x}", key.module_id.package_id.raw());
        let module = format!("{:08x}", key.module_id.local_id);
        let file_version = key.file_version.0;
        let profile_id = key
            .profile_id
            .map(|id| id.raw().to_string())
            .unwrap_or_else(|| "*".to_string());
        let profile_version = key
            .profile_version
            .map(|version| version.0.to_string())
            .unwrap_or_else(|| "*".to_string());
        let file_name = format!(
            "{kind_dir}-f{file_version}-p{profile_id}-pv{profile_version}-s{source_hash:016x}-c{config_hash:016x}-t{target_hash:016x}-d{dependency_hash:016x}.bin",
            source_hash = key.source_hash,
            config_hash = key.config_hash,
            target_hash = key.target_hash,
            dependency_hash = key.dependency_hash,
        );

        // build the cache path
        root.join(DEFAULT_COMPILER_CACHE_NAMESPACE)
            .join(kind_dir)
            .join(package)
            .join(module)
            .join(file_name)
    }

    /// Build the cache access path for a cache entry.
    pub(super) fn cache_access_path(&self, path: &Path) -> PathBuf {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("cache");
        let access_name = format!("{file_name}{CACHE_ACCESS_SUFFIX}");
        path.with_file_name(access_name)
    }

    /// Evict cache entries when size limits are exceeded.
    pub(super) fn evict_cache_entries(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
    ) -> Result<(), CacheError> {
        // skip when no size limit is configured
        let Some(max_size_mb) = options.max_size_mb else {
            return Ok(());
        };

        // skip when the configured limit is zero
        if max_size_mb == 0 {
            return Ok(());
        }

        if cache_store.kind() != CacheStoreKind::Disk {
            return Ok(());
        }

        // compute the size limit in bytes
        let limit_bytes = max_size_mb.saturating_mul(BYTES_PER_MB);
        // resolve the cache root directory
        let root = options.dir.join(DEFAULT_COMPILER_CACHE_NAMESPACE);
        if !root.exists() {
            return Ok(());
        }

        // collect cache entries for eviction
        let mut entries = Vec::new();
        self.collect_cache_entries(&root, &mut entries)?;

        // compute total cache size for eviction
        let mut total_size = entries.iter().map(|entry| entry.size).sum::<u64>();
        self.disk_state
            .size_bytes
            .store(total_size, Ordering::Relaxed);
        self.disk_state.initialized.store(true, Ordering::Relaxed);

        if total_size <= limit_bytes {
            return Ok(());
        }

        // sort entries by eviction key
        entries.sort_by_key(|entry| entry.eviction_key(options.policy));

        // evict oldest entries until under limit
        for entry in entries {
            if total_size <= limit_bytes {
                break;
            }

            // remove cache entry from disk
            match std::fs::remove_file(&entry.path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(CacheError::Io(error)),
            }
            let access_path = self.cache_access_path(&entry.path);
            let _ = std::fs::remove_file(access_path);

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

            // skip access marker files
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(CACHE_ACCESS_SUFFIX))
            {
                continue;
            }

            // collect cache file entry metadata
            let accessed = metadata.accessed().ok();
            let modified = metadata.modified().ok();
            let access_path = self.cache_access_path(&path);
            let access_modified = std::fs::metadata(access_path)
                .ok()
                .and_then(|metadata| metadata.modified().ok());
            entries.push(CacheFileEntry {
                path,
                size: metadata.len(),
                accessed: access_modified.or(accessed),
                modified,
            });
        }

        Ok(())
    }
}
