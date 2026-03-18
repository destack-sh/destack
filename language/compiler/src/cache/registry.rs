use dashmap::DashMap;

use destack_source::{CacheKind, FileVersion, ModuleId, ProfileId, ProfileVersion};
use destack_workspace::{
    Ast, AstCacheEntry, CacheError, CacheStore, DirAnalyzed, DirAnalyzedCacheEntry, DirBase,
    DirBaseCacheEntry, DirCacheEntry, DirPatched, DirPatchedCacheEntry, DirPrepared,
    DirPreparedCacheEntry, DirResolved, DirResolvedCacheEntry, MirBase, MirBaseCacheEntry,
};

use super::disk::CacheDiskState;
use super::handle::{CacheReadOutcome, CacheReadSource};
use super::memory::{CacheMemoryEntry, CacheMemoryState};
use super::{CacheContext, CacheOptions};

/// Cache key for in memory entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// The cache kind.
    pub cache_kind: CacheKind,
    /// The module id.
    pub module_id: ModuleId,
    /// The file version.
    pub file_version: FileVersion,
    /// The profile id when applicable.
    pub profile_id: Option<ProfileId>,
    /// The profile version when applicable.
    pub profile_version: Option<ProfileVersion>,
    /// The source hash.
    pub source_hash: u64,
    /// The config hash.
    pub config_hash: u64,
    /// The target hash.
    pub target_hash: u64,
    /// The dependency hash.
    pub dependency_hash: u64,
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
            dependency_hash: context.dependency_hash,
        }
    }
}

/// In memory cache registry for compiler artifacts.
#[derive(Debug)]
pub struct CacheRegistry {
    /// Cached AST entries.
    pub(super) ast: DashMap<CacheKey, CacheMemoryEntry<AstCacheEntry>>,
    /// Cached base DIR entries.
    pub(super) dir_base: DashMap<CacheKey, CacheMemoryEntry<DirBaseCacheEntry>>,
    /// Cached prepared DIR entries.
    pub(super) dir_prepared: DashMap<CacheKey, CacheMemoryEntry<DirPreparedCacheEntry>>,
    /// Cached resolved DIR entries.
    pub(super) dir_resolved: DashMap<CacheKey, CacheMemoryEntry<DirResolvedCacheEntry>>,
    /// Cached analyzed DIR entries.
    pub(super) dir_analyzed: DashMap<CacheKey, CacheMemoryEntry<DirAnalyzedCacheEntry>>,
    /// Cached patched DIR entries.
    pub(super) dir_patched: DashMap<CacheKey, CacheMemoryEntry<DirPatchedCacheEntry>>,
    /// Cached MIR entries.
    pub(super) mir: DashMap<CacheKey, CacheMemoryEntry<MirBaseCacheEntry>>,
    /// Memory cache tracking state.
    pub(super) memory_state: CacheMemoryState,
    /// Disk cache tracking state.
    pub(super) disk_state: CacheDiskState,
}

impl CacheRegistry {
    /// Create a new cache registry.
    pub fn new() -> Self {
        Self {
            ast: DashMap::new(),
            dir_base: DashMap::new(),
            dir_prepared: DashMap::new(),
            dir_resolved: DashMap::new(),
            dir_analyzed: DashMap::new(),
            dir_patched: DashMap::new(),
            mir: DashMap::new(),
            memory_state: CacheMemoryState::new(),
            disk_state: CacheDiskState::new(),
        }
    }

    /// Read an AST cache entry if available.
    pub fn read_ast_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<AstCacheEntry>, CacheError> {
        // read entry outcome from cache
        let outcome = self.read_ast_cache_outcome(cache_store, options, context, module_id)?;

        // map cache outcome to response
        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write an AST cache entry.
    pub fn write_ast_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: Ast,
    ) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(());
        }

        // build the entry and store in memory
        let header = context.header(CacheKind::Ast, module_id);
        let entry = if options.is_disk_enabled() {
            AstCacheEntry::new(header, payload)?
        } else {
            AstCacheEntry::new_unchecked(header, payload)
        };
        let key = CacheKey::new(CacheKind::Ast, module_id, context);
        let size_bytes = self.entry_size_bytes(options, &entry)?;
        self.insert_memory_entry(
            &self.ast,
            key.clone(),
            CacheMemoryEntry::new(entry.clone(), size_bytes),
        );
        self.evict_memory_entries(options)?;

        // stop when disk cache is disabled
        if !options.is_disk_enabled() {
            return Ok(());
        }

        // write to disk
        self.write_cache_entry(cache_store, options, &key, &entry)
    }

    /// Read a base DIR cache entry if available.
    pub fn read_dir_base_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<DirBaseCacheEntry>, CacheError> {
        let outcome = self.read_dir_base_cache_outcome(cache_store, options, context, module_id)?;

        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write a base DIR cache entry.
    pub fn write_dir_base_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: DirBase,
    ) -> Result<(), CacheError> {
        self.write_dir_cache_entry(
            &self.dir_base,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirBase,
            payload,
        )
    }

    /// Read a prepared DIR cache entry if available.
    pub fn read_dir_prepared_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<DirPreparedCacheEntry>, CacheError> {
        let outcome =
            self.read_dir_prepared_cache_outcome(cache_store, options, context, module_id)?;

        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write a prepared DIR cache entry.
    pub fn write_dir_prepared_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: DirPrepared,
    ) -> Result<(), CacheError> {
        self.write_dir_cache_entry(
            &self.dir_prepared,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirPrepared,
            payload,
        )
    }

    /// Read a resolved DIR cache entry if available.
    pub fn read_dir_resolved_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<DirResolvedCacheEntry>, CacheError> {
        let outcome =
            self.read_dir_resolved_cache_outcome(cache_store, options, context, module_id)?;

        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write a resolved DIR cache entry.
    pub fn write_dir_resolved_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: DirResolved,
    ) -> Result<(), CacheError> {
        self.write_dir_cache_entry(
            &self.dir_resolved,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirResolved,
            payload,
        )
    }

    /// Read an analyzed DIR cache entry if available.
    pub fn read_dir_analyzed_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<DirAnalyzedCacheEntry>, CacheError> {
        let outcome =
            self.read_dir_analyzed_cache_outcome(cache_store, options, context, module_id)?;

        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write an analyzed DIR cache entry.
    pub fn write_dir_analyzed_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: DirAnalyzed,
    ) -> Result<(), CacheError> {
        self.write_dir_cache_entry(
            &self.dir_analyzed,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirAnalyzed,
            payload,
        )
    }

    /// Read a patched DIR cache entry if available.
    pub fn read_dir_patched_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<DirPatchedCacheEntry>, CacheError> {
        let outcome =
            self.read_dir_patched_cache_outcome(cache_store, options, context, module_id)?;

        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write a patched DIR cache entry.
    pub fn write_dir_patched_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: DirPatched,
    ) -> Result<(), CacheError> {
        self.write_dir_cache_entry(
            &self.dir_patched,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirPatched,
            payload,
        )
    }

    /// Read a MIR cache entry if available.
    pub fn read_mir_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<MirBaseCacheEntry>, CacheError> {
        // read entry outcome from cache
        let outcome = self.read_mir_cache_outcome(cache_store, options, context, module_id)?;

        // map cache outcome to response
        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Read an AST cache entry with its source.
    pub(super) fn read_ast_cache_outcome(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<AstCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try in memory cache first
        let key = CacheKey::new(CacheKind::Ast, module_id, context);
        if let Some(mut entry_ref) = self.ast.get_mut(&key) {
            let entry = entry_ref.entry.clone();
            match self.validate_ast_entry(&entry, context, options) {
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
        if !options.is_disk_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try disk cache
        let path = self.cache_entry_path(options, &key);
        if !cache_store.exists(&path).map_err(CacheError::from)? {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: AstCacheEntry = match self.read_disk_entry(cache_store, options, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        };
        match self.validate_ast_entry(&entry, context, options) {
            Ok(true) => {
                if let Ok(size_bytes) = self.entry_size_bytes(options, &entry) {
                    self.insert_memory_entry(
                        &self.ast,
                        key,
                        CacheMemoryEntry::new(entry.clone(), size_bytes),
                    );
                    self.evict_memory_entries(options)?;
                }
                self.touch_disk_entry(cache_store, options, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Read a base DIR cache entry with its source.
    pub(super) fn read_dir_base_cache_outcome(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<DirBaseCacheEntry>, CacheError> {
        self.read_dir_cache_outcome(
            &self.dir_base,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirBase,
        )
    }

    /// Read a prepared DIR cache entry with its source.
    pub(super) fn read_dir_prepared_cache_outcome(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<DirPreparedCacheEntry>, CacheError> {
        self.read_dir_cache_outcome(
            &self.dir_prepared,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirPrepared,
        )
    }

    /// Read a resolved DIR cache entry with its source.
    pub(super) fn read_dir_resolved_cache_outcome(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<DirResolvedCacheEntry>, CacheError> {
        self.read_dir_cache_outcome(
            &self.dir_resolved,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirResolved,
        )
    }

    /// Read an analyzed DIR cache entry with its source.
    pub(super) fn read_dir_analyzed_cache_outcome(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<DirAnalyzedCacheEntry>, CacheError> {
        self.read_dir_cache_outcome(
            &self.dir_analyzed,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirAnalyzed,
        )
    }

    /// Read a patched DIR cache entry with its source.
    pub(super) fn read_dir_patched_cache_outcome(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<DirPatchedCacheEntry>, CacheError> {
        self.read_dir_cache_outcome(
            &self.dir_patched,
            cache_store,
            options,
            context,
            module_id,
            CacheKind::DirPatched,
        )
    }

    /// Read a DIR cache entry with its source for one exact stage.
    fn read_dir_cache_outcome<T>(
        &self,
        map: &DashMap<CacheKey, CacheMemoryEntry<DirCacheEntry<T>>>,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        cache_kind: CacheKind,
    ) -> Result<CacheReadOutcome<DirCacheEntry<T>>, CacheError>
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try in memory cache first
        let key = CacheKey::new(cache_kind, module_id, context);
        if let Some(mut entry_ref) = map.get_mut(&key) {
            let entry = entry_ref.entry.clone();
            match self.validate_dir_entry(&entry, context, options, cache_kind) {
                Ok(true) => {
                    entry_ref.touch();
                    return Ok(CacheReadOutcome::Hit {
                        entry,
                        source: CacheReadSource::Memory,
                    });
                }
                Ok(false) => {
                    drop(entry_ref);
                    self.remove_memory_entry(map, &key);
                }
                Err(_) => {
                    drop(entry_ref);
                    self.remove_memory_entry(map, &key);
                    return Ok(CacheReadOutcome::Error);
                }
            }
        }

        // stop when disk cache is disabled
        if !options.is_disk_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try disk cache
        let path = self.cache_entry_path(options, &key);
        if !cache_store.exists(&path).map_err(CacheError::from)? {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: DirCacheEntry<T> = match self.read_disk_entry(cache_store, options, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        };
        match self.validate_dir_entry(&entry, context, options, cache_kind) {
            Ok(true) => {
                if let Ok(size_bytes) = self.entry_size_bytes(options, &entry) {
                    self.insert_memory_entry(
                        map,
                        key,
                        CacheMemoryEntry::new(entry.clone(), size_bytes),
                    );
                    self.evict_memory_entries(options)?;
                }
                self.touch_disk_entry(cache_store, options, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Write one DIR cache entry for one exact stage.
    fn write_dir_cache_entry<T>(
        &self,
        map: &DashMap<CacheKey, CacheMemoryEntry<DirCacheEntry<T>>>,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        cache_kind: CacheKind,
        payload: T,
    ) -> Result<(), CacheError>
    where
        T: Clone + serde::Serialize,
    {
        if !options.is_enabled() {
            return Ok(());
        }

        let header = context.header(cache_kind, module_id);
        let entry = if options.is_disk_enabled() {
            DirCacheEntry::new(header, payload)?
        } else {
            DirCacheEntry::new_unchecked(header, payload)
        };
        let key = CacheKey::new(cache_kind, module_id, context);
        let size_bytes = self.entry_size_bytes(options, &entry)?;
        self.insert_memory_entry(
            map,
            key.clone(),
            CacheMemoryEntry::new(entry.clone(), size_bytes),
        );
        self.evict_memory_entries(options)?;

        if !options.is_disk_enabled() {
            return Ok(());
        }

        self.write_cache_entry(cache_store, options, &key, &entry)
    }

    /// Read a MIR cache entry with its source.
    pub(super) fn read_mir_cache_outcome(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<MirBaseCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try in memory cache first
        let key = CacheKey::new(CacheKind::Mir, module_id, context);
        if let Some(mut entry_ref) = self.mir.get_mut(&key) {
            let entry = entry_ref.entry.clone();
            match self.validate_mir_entry(&entry, context, options) {
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
        if !options.is_disk_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try disk cache
        let path = self.cache_entry_path(options, &key);
        if !cache_store.exists(&path).map_err(CacheError::from)? {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: MirBaseCacheEntry = match self.read_disk_entry(cache_store, options, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        };
        match self.validate_mir_entry(&entry, context, options) {
            Ok(true) => {
                if let Ok(size_bytes) = self.entry_size_bytes(options, &entry) {
                    self.insert_memory_entry(
                        &self.mir,
                        key,
                        CacheMemoryEntry::new(entry.clone(), size_bytes),
                    );
                    self.evict_memory_entries(options)?;
                }
                self.touch_disk_entry(cache_store, options, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(cache_store, options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Write a MIR cache entry.
    pub fn write_mir_cache(
        &self,
        cache_store: &dyn CacheStore,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: MirBase,
    ) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(());
        }

        // build the entry and store in memory
        let header = context.header(CacheKind::Mir, module_id);
        let entry = if options.is_disk_enabled() {
            MirBaseCacheEntry::new(header, payload)?
        } else {
            MirBaseCacheEntry::new_unchecked(header, payload)
        };
        let key = CacheKey::new(CacheKind::Mir, module_id, context);
        let size_bytes = self.entry_size_bytes(options, &entry)?;
        self.insert_memory_entry(
            &self.mir,
            key.clone(),
            CacheMemoryEntry::new(entry.clone(), size_bytes),
        );
        self.evict_memory_entries(options)?;

        // stop when disk cache is disabled
        if !options.is_disk_enabled() {
            return Ok(());
        }

        // write to disk
        self.write_cache_entry(cache_store, options, &key, &entry)
    }
}

impl Default for CacheRegistry {
    fn default() -> Self {
        Self::new()
    }
}
