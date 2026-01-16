use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use filetime::{FileTime, set_file_mtime};
use fs2::FileExt;

use destack_source::{CacheHeader, CacheKind};
use destack_workspace::{
    CACHE_ENTRY_LIMIT_BYTES, CacheError, CachePolicy, CacheValidate, ModuleAstCacheEntry,
    ModuleDirCacheEntry, ModuleMirCacheEntry, read_cache_entry, write_cache_entry,
};

use super::context::{BYTES_PER_MB, DEFAULT_CACHE_NAMESPACE};
use super::{CacheContext, CacheKey, CacheOptions, CacheRegistry};

const CACHE_LOCK_FILE: &str = "cache.lock";

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

impl CacheRegistry {
    /// Read a cache entry from disk with size limits and locking.
    pub(super) fn read_disk_entry<T: serde::de::DeserializeOwned>(
        &self,
        options: &CacheOptions,
        path: &Path,
    ) -> Result<T, CacheError> {
        let _lock = self.lock_cache_shared(options)?;
        self.check_disk_entry_size(path)?;
        read_cache_entry(path)
    }

    /// Remove a cache entry from disk and update size tracking.
    pub(super) fn remove_disk_entry(
        &self,
        options: &CacheOptions,
        path: &Path,
    ) -> Result<(), CacheError> {
        let _lock = self.lock_cache_exclusive(options)?;
        let size = if options.max_size_mb.is_some() {
            std::fs::metadata(path)
                .map(|metadata| metadata.len())
                .unwrap_or_default()
        } else {
            0
        };
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(CacheError::Io(error)),
        }

        if size > 0 {
            self.decrement_disk_size(size);
        }
        Ok(())
    }

    /// Ensure the disk cache size is tracked before eviction.
    fn ensure_disk_size_initialized(&self, options: &CacheOptions) -> Result<(), CacheError> {
        if self.disk_state.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }

        let root = options.dir.join(DEFAULT_CACHE_NAMESPACE);
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
    pub(super) fn touch_disk_entry(&self, options: &CacheOptions, path: &Path) {
        if options.policy != CachePolicy::Lru {
            return;
        }

        let now = FileTime::from_system_time(SystemTime::now());
        let _ = set_file_mtime(path, now);
    }

    /// Acquire a shared disk cache lock.
    fn lock_cache_shared(
        &self,
        options: &CacheOptions,
    ) -> Result<Option<std::fs::File>, CacheError> {
        let root = options.dir.join(DEFAULT_CACHE_NAMESPACE);
        if !root.exists() {
            return Ok(None);
        }
        let path = root.join(CACHE_LOCK_FILE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(CacheError::Io)?;
        FileExt::lock_shared(&file).map_err(CacheError::Io)?;
        Ok(Some(file))
    }

    /// Acquire an exclusive disk cache lock.
    fn lock_cache_exclusive(
        &self,
        options: &CacheOptions,
    ) -> Result<Option<std::fs::File>, CacheError> {
        let root = options.dir.join(DEFAULT_CACHE_NAMESPACE);
        std::fs::create_dir_all(&root).map_err(CacheError::Io)?;
        let path = root.join(CACHE_LOCK_FILE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(CacheError::Io)?;
        FileExt::lock_exclusive(&file).map_err(CacheError::Io)?;
        Ok(Some(file))
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

    /// Validate a DIR cache entry against the context.
    pub(super) fn validate_dir_entry(
        &self,
        entry: &ModuleDirCacheEntry,
        context: &CacheContext,
        options: &CacheOptions,
    ) -> Result<bool, CacheError> {
        // compare headers first
        let expected = context.header(CacheKind::Dir, entry.header.module_id);
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
    }

    /// Write a cache entry to disk after ensuring its directory exists.
    pub(super) fn write_cache_entry<T: serde::Serialize>(
        &self,
        options: &CacheOptions,
        key: &CacheKey,
        entry: &T,
    ) -> Result<(), CacheError> {
        let _lock = self.lock_cache_exclusive(options)?;

        // ensure cache directory exists
        let path = self.cache_entry_path_from_root(&options.dir, key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(CacheError::Io)?;
        }

        // capture size before write
        let previous_size = std::fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or_default();

        let track_disk_size = options.max_size_mb.is_some();
        if track_disk_size {
            self.ensure_disk_size_initialized(options)?;
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
        self.evict_cache_entries(options)?;

        Ok(())
    }

    /// Build the cache entry path for a key.
    pub(super) fn cache_entry_path(&self, options: &CacheOptions, key: &CacheKey) -> PathBuf {
        self.cache_entry_path_from_root(&options.dir, key)
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
    pub(super) fn evict_cache_entries(&self, options: &CacheOptions) -> Result<(), CacheError> {
        // skip when no size limit is configured
        let Some(max_size_mb) = options.max_size_mb else {
            return Ok(());
        };

        // skip when the configured limit is zero
        if max_size_mb == 0 {
            return Ok(());
        }

        // compute the size limit in bytes
        let limit_bytes = max_size_mb.saturating_mul(BYTES_PER_MB);
        self.ensure_disk_size_initialized(options)?;
        let mut total_size = self.disk_state.size_bytes.load(Ordering::Relaxed);
        if total_size <= limit_bytes {
            return Ok(());
        }

        // resolve the cache root directory
        let root = options.dir.join(DEFAULT_CACHE_NAMESPACE);
        if !root.exists() {
            return Ok(());
        }

        // collect cache entries for eviction
        let mut entries = Vec::new();
        self.collect_cache_entries(&root, &mut entries)?;

        // sum cache size across entries
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
