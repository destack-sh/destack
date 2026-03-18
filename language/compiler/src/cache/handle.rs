use destack_source::{CacheKind, ModuleId};
use destack_workspace::{
    Ast, AstCacheEntry, CacheError, CacheStore, DirAnalyzed, DirAnalyzedCacheEntry, DirBase,
    DirBaseCacheEntry, DirCacheEntry, DirPatched, DirPatchedCacheEntry, DirPrepared,
    DirPreparedCacheEntry, DirResolved, DirResolvedCacheEntry, MirBase, MirBaseCacheEntry,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::compile::CompilerStats;

use super::{CacheContext, CacheOptions, CacheRegistry};

/// Cache read source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CacheReadSource {
    /// Cache entry was loaded from memory.
    Memory,
    /// Cache entry was loaded from disk.
    Disk,
}

/// Cache read outcome.
#[derive(Debug)]
pub(super) enum CacheReadOutcome<T> {
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
    pub(super) registry: &'a CacheRegistry,
    /// The cache store.
    pub(super) cache_store: &'a dyn CacheStore,
    /// The cache stats.
    pub(super) stats: &'a CompilerStats,
    /// The cache options.
    pub(super) options: CacheOptions,
    /// The cache context.
    pub(super) context: CacheContext,
    /// The module id.
    pub(super) module_id: ModuleId,
}

impl CacheHandle<'_> {
    /// Read an AST cache entry if available.
    pub fn read_ast(&self) -> Result<Option<AstCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !self.options.is_enabled() {
            return Ok(None);
        }

        // read from cache registry
        let outcome = self.registry.read_ast_cache_outcome(
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
        );

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

    /// Read a base DIR cache entry if available.
    pub fn read_dir_base(&self) -> Result<Option<DirBaseCacheEntry>, CacheError> {
        self.read_dir_entry(
            CacheKind::DirBase,
            |registry, cache_store, options, context, module_id| {
                registry.read_dir_base_cache_outcome(cache_store, options, context, module_id)
            },
        )
    }

    /// Read a prepared DIR cache entry if available.
    pub fn read_dir_prepared(&self) -> Result<Option<DirPreparedCacheEntry>, CacheError> {
        self.read_dir_entry(
            CacheKind::DirPrepared,
            |registry, cache_store, options, context, module_id| {
                registry.read_dir_prepared_cache_outcome(cache_store, options, context, module_id)
            },
        )
    }

    /// Read a resolved DIR cache entry if available.
    pub fn read_dir_resolved(&self) -> Result<Option<DirResolvedCacheEntry>, CacheError> {
        self.read_dir_entry(
            CacheKind::DirResolved,
            |registry, cache_store, options, context, module_id| {
                registry.read_dir_resolved_cache_outcome(cache_store, options, context, module_id)
            },
        )
    }

    /// Read an analyzed DIR cache entry if available.
    pub fn read_dir_analyzed(&self) -> Result<Option<DirAnalyzedCacheEntry>, CacheError> {
        self.read_dir_entry(
            CacheKind::DirAnalyzed,
            |registry, cache_store, options, context, module_id| {
                registry.read_dir_analyzed_cache_outcome(cache_store, options, context, module_id)
            },
        )
    }

    /// Read a patched DIR cache entry if available.
    pub fn read_dir_patched(&self) -> Result<Option<DirPatchedCacheEntry>, CacheError> {
        self.read_dir_entry(
            CacheKind::DirPatched,
            |registry, cache_store, options, context, module_id| {
                registry.read_dir_patched_cache_outcome(cache_store, options, context, module_id)
            },
        )
    }

    /// Read a MIR cache entry if available.
    pub fn read_mir(&self) -> Result<Option<MirBaseCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !self.options.is_enabled() {
            return Ok(None);
        }

        // read from cache registry
        let outcome = self.registry.read_mir_cache_outcome(
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
        );

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

    /// Write an AST cache entry if enabled.
    pub fn write_ast(&self, payload: Ast) -> Result<(), CacheError> {
        self.registry.write_ast_cache(
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
            payload,
        )
    }

    /// Write a base DIR cache entry if enabled.
    pub fn write_dir_base(&self, payload: DirBase) -> Result<(), CacheError> {
        self.write_dir_entry(
            |registry, cache_store, options, context, module_id, payload| {
                registry.write_dir_base_cache(cache_store, options, context, module_id, payload)
            },
            payload,
        )
    }

    /// Write a prepared DIR cache entry if enabled.
    pub fn write_dir_prepared(&self, payload: DirPrepared) -> Result<(), CacheError> {
        self.write_dir_entry(
            |registry, cache_store, options, context, module_id, payload| {
                registry.write_dir_prepared_cache(cache_store, options, context, module_id, payload)
            },
            payload,
        )
    }

    /// Write a resolved DIR cache entry if enabled.
    pub fn write_dir_resolved(&self, payload: DirResolved) -> Result<(), CacheError> {
        self.write_dir_entry(
            |registry, cache_store, options, context, module_id, payload| {
                registry.write_dir_resolved_cache(cache_store, options, context, module_id, payload)
            },
            payload,
        )
    }

    /// Write an analyzed DIR cache entry if enabled.
    pub fn write_dir_analyzed(&self, payload: DirAnalyzed) -> Result<(), CacheError> {
        self.write_dir_entry(
            |registry, cache_store, options, context, module_id, payload| {
                registry.write_dir_analyzed_cache(cache_store, options, context, module_id, payload)
            },
            payload,
        )
    }

    /// Write a patched DIR cache entry if enabled.
    pub fn write_dir_patched(&self, payload: DirPatched) -> Result<(), CacheError> {
        self.write_dir_entry(
            |registry, cache_store, options, context, module_id, payload| {
                registry.write_dir_patched_cache(cache_store, options, context, module_id, payload)
            },
            payload,
        )
    }

    /// Write a MIR cache entry if enabled.
    pub fn write_mir(&self, payload: MirBase) -> Result<(), CacheError> {
        self.registry.write_mir_cache(
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
            payload,
        )
    }

    /// Record cache hit statistics for a cache kind.
    fn record_cache_hit(&self, source: CacheReadSource, cache_kind: CacheKind) {
        match cache_kind {
            CacheKind::Ast => match source {
                CacheReadSource::Memory => self.stats.record_cache_ast_hit_memory(),
                CacheReadSource::Disk => self.stats.record_cache_ast_hit_disk(),
            },
            CacheKind::DirBase
            | CacheKind::DirPrepared
            | CacheKind::DirResolved
            | CacheKind::DirAnalyzed
            | CacheKind::DirPatched => match source {
                CacheReadSource::Memory => self.stats.record_cache_dir_hit_memory(),
                CacheReadSource::Disk => self.stats.record_cache_dir_hit_disk(),
            },
            CacheKind::Mir => match source {
                CacheReadSource::Memory => self.stats.record_cache_mir_hit_memory(),
                CacheReadSource::Disk => self.stats.record_cache_mir_hit_disk(),
            },
        }
    }

    /// Read one exact DIR cache entry.
    fn read_dir_entry<T, F>(
        &self,
        cache_kind: CacheKind,
        read: F,
    ) -> Result<Option<DirCacheEntry<T>>, CacheError>
    where
        T: Clone + Serialize + DeserializeOwned,
        F: FnOnce(
            &CacheRegistry,
            &dyn CacheStore,
            &CacheOptions,
            &CacheContext,
            ModuleId,
        ) -> Result<CacheReadOutcome<DirCacheEntry<T>>, CacheError>,
    {
        // skip when cache is disabled
        if !self.options.is_enabled() {
            return Ok(None);
        }

        // read the cache entry and record the outcome
        let outcome = read(
            self.registry,
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
        );

        match &outcome {
            Ok(CacheReadOutcome::Hit { source, .. }) => {
                self.record_cache_hit(*source, cache_kind);
            }
            Ok(CacheReadOutcome::Miss) => {
                self.stats.record_cache_dir_miss();
            }
            Ok(CacheReadOutcome::Error) | Err(_) => {
                self.stats.record_cache_error();
            }
        }

        match outcome {
            Ok(CacheReadOutcome::Hit { entry, .. }) => Ok(Some(entry)),
            Ok(CacheReadOutcome::Miss) | Ok(CacheReadOutcome::Error) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// Write one exact DIR cache entry.
    fn write_dir_entry<T, F>(&self, write: F, payload: T) -> Result<(), CacheError>
    where
        T: Clone + Serialize + DeserializeOwned,
        F: FnOnce(
            &CacheRegistry,
            &dyn CacheStore,
            &CacheOptions,
            &CacheContext,
            ModuleId,
            T,
        ) -> Result<(), CacheError>,
    {
        write(
            self.registry,
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
            payload,
        )
    }
}
