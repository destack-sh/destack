use dashmap::DashMap;

use destack_source::{CacheKind, FileVersion, ModuleId, ProfileId, ProfileVersion};
use destack_workspace::{
    CacheError, ModuleAstCacheEntry, ModuleAstData, ModuleDirCacheEntry, ModuleDirData,
    ModuleMirCacheEntry, ModuleMirData,
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
    pub(super) ast: DashMap<CacheKey, CacheMemoryEntry<ModuleAstCacheEntry>>,
    /// Cached DIR entries.
    pub(super) dir: DashMap<CacheKey, CacheMemoryEntry<ModuleDirCacheEntry>>,
    /// Cached MIR entries.
    pub(super) mir: DashMap<CacheKey, CacheMemoryEntry<ModuleMirCacheEntry>>,
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
            dir: DashMap::new(),
            mir: DashMap::new(),
            memory_state: CacheMemoryState::new(),
            disk_state: CacheDiskState::new(),
        }
    }

    /// Read an AST cache entry if available.
    pub fn read_ast_cache(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<ModuleAstCacheEntry>, CacheError> {
        // read entry outcome from cache
        let outcome = self.read_ast_cache_outcome(options, context, module_id)?;

        // map cache outcome to response
        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write an AST cache entry.
    pub fn write_ast_cache(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: ModuleAstData,
    ) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(());
        }

        // build the entry and store in memory
        let header = context.header(CacheKind::Ast, module_id);
        let entry = if options.is_disk_enabled() {
            ModuleAstCacheEntry::new(header, payload)?
        } else {
            ModuleAstCacheEntry::new_unchecked(header, payload)
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
        self.write_cache_entry(options, &key, &entry)
    }

    /// Read a DIR cache entry if available.
    pub fn read_dir_cache(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        // read entry outcome from cache
        let outcome = self.read_dir_cache_outcome(options, context, module_id)?;

        // map cache outcome to response
        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Write a DIR cache entry.
    pub fn write_dir_cache(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: ModuleDirData,
    ) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(());
        }

        // build the entry and store in memory
        let header = context.header(CacheKind::Dir, module_id);
        let entry = if options.is_disk_enabled() {
            ModuleDirCacheEntry::new(header, payload)?
        } else {
            ModuleDirCacheEntry::new_unchecked(header, payload)
        };
        let key = CacheKey::new(CacheKind::Dir, module_id, context);
        let size_bytes = self.entry_size_bytes(options, &entry)?;
        self.insert_memory_entry(
            &self.dir,
            key.clone(),
            CacheMemoryEntry::new(entry.clone(), size_bytes),
        );
        self.evict_memory_entries(options)?;

        // stop when disk cache is disabled
        if !options.is_disk_enabled() {
            return Ok(());
        }

        // write to disk
        self.write_cache_entry(options, &key, &entry)
    }

    /// Read a MIR cache entry if available.
    pub fn read_mir_cache(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<Option<ModuleMirCacheEntry>, CacheError> {
        // read entry outcome from cache
        let outcome = self.read_mir_cache_outcome(options, context, module_id)?;

        // map cache outcome to response
        match outcome {
            CacheReadOutcome::Hit { entry, .. } => Ok(Some(entry)),
            CacheReadOutcome::Miss | CacheReadOutcome::Error => Ok(None),
        }
    }

    /// Read an AST cache entry with its source.
    pub(super) fn read_ast_cache_outcome(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<ModuleAstCacheEntry>, CacheError> {
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
        if !path.exists() {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: ModuleAstCacheEntry = match self.read_disk_entry(options, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(options, &path).ok();
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
                self.touch_disk_entry(options, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(options, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Read a DIR cache entry with its source.
    pub(super) fn read_dir_cache_outcome(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<ModuleDirCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try in memory cache first
        let key = CacheKey::new(CacheKind::Dir, module_id, context);
        if let Some(mut entry_ref) = self.dir.get_mut(&key) {
            let entry = entry_ref.entry.clone();
            match self.validate_dir_entry(&entry, context, options) {
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
        if !options.is_disk_enabled() {
            return Ok(CacheReadOutcome::Miss);
        }

        // try disk cache
        let path = self.cache_entry_path(options, &key);
        if !path.exists() {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: ModuleDirCacheEntry = match self.read_disk_entry(options, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        };
        match self.validate_dir_entry(&entry, context, options) {
            Ok(true) => {
                if let Ok(size_bytes) = self.entry_size_bytes(options, &entry) {
                    self.insert_memory_entry(
                        &self.dir,
                        key,
                        CacheMemoryEntry::new(entry.clone(), size_bytes),
                    );
                    self.evict_memory_entries(options)?;
                }
                self.touch_disk_entry(options, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(options, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Read a MIR cache entry with its source.
    pub(super) fn read_mir_cache_outcome(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
    ) -> Result<CacheReadOutcome<ModuleMirCacheEntry>, CacheError> {
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
        if !path.exists() {
            return Ok(CacheReadOutcome::Miss);
        }
        let entry: ModuleMirCacheEntry = match self.read_disk_entry(options, &path) {
            Ok(entry) => entry,
            Err(_) => {
                self.remove_disk_entry(options, &path).ok();
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
                self.touch_disk_entry(options, &path);
                return Ok(CacheReadOutcome::Hit {
                    entry,
                    source: CacheReadSource::Disk,
                });
            }
            Ok(false) => {
                self.remove_disk_entry(options, &path).ok();
            }
            Err(_) => {
                self.remove_disk_entry(options, &path).ok();
                return Ok(CacheReadOutcome::Error);
            }
        }

        Ok(CacheReadOutcome::Miss)
    }

    /// Write a MIR cache entry.
    pub fn write_mir_cache(
        &self,
        options: &CacheOptions,
        context: &CacheContext,
        module_id: ModuleId,
        payload: ModuleMirData,
    ) -> Result<(), CacheError> {
        // skip when cache is disabled
        if !options.is_enabled() {
            return Ok(());
        }

        // build the entry and store in memory
        let header = context.header(CacheKind::Mir, module_id);
        let entry = if options.is_disk_enabled() {
            ModuleMirCacheEntry::new(header, payload)?
        } else {
            ModuleMirCacheEntry::new_unchecked(header, payload)
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
        self.write_cache_entry(options, &key, &entry)
    }
}

impl Default for CacheRegistry {
    fn default() -> Self {
        Self::new()
    }
}
