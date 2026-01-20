use destack_source::{CacheKind, ModuleId};
use destack_workspace::{
    CacheError, CacheStore, ModuleAstCacheEntry, ModuleAstData, ModuleDirCacheEntry, ModuleDirData,
    ModuleMirCacheEntry, ModuleMirData,
};

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
    pub fn read_ast(&self) -> Result<Option<ModuleAstCacheEntry>, CacheError> {
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
    pub fn read_dir_base(&self) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        self.read_dir_with_kind(CacheKind::DirBase)
    }

    /// Read a resolved DIR cache entry if available.
    pub fn read_dir_resolved(&self) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        self.read_dir_with_kind(CacheKind::DirResolved)
    }

    /// Read an analyzed DIR cache entry if available.
    pub fn read_dir_analyzed(&self) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        self.read_dir_with_kind(CacheKind::DirAnalyzed)
    }

    /// Read an executed DIR cache entry if available.
    pub fn read_dir_executed(&self) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        self.read_dir_with_kind(CacheKind::DirExecuted)
    }

    /// Read a MIR cache entry if available.
    pub fn read_mir(&self) -> Result<Option<ModuleMirCacheEntry>, CacheError> {
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
    pub fn write_ast(&self, payload: ModuleAstData) -> Result<(), CacheError> {
        self.registry.write_ast_cache(
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
            payload,
        )
    }

    /// Write a base DIR cache entry if enabled.
    pub fn write_dir_base(&self, payload: ModuleDirData) -> Result<(), CacheError> {
        self.write_dir_with_kind(CacheKind::DirBase, payload)
    }

    /// Write a resolved DIR cache entry if enabled.
    pub fn write_dir_resolved(&self, payload: ModuleDirData) -> Result<(), CacheError> {
        self.write_dir_with_kind(CacheKind::DirResolved, payload)
    }

    /// Write an analyzed DIR cache entry if enabled.
    pub fn write_dir_analyzed(&self, payload: ModuleDirData) -> Result<(), CacheError> {
        self.write_dir_with_kind(CacheKind::DirAnalyzed, payload)
    }

    /// Write an executed DIR cache entry if enabled.
    pub fn write_dir_executed(&self, payload: ModuleDirData) -> Result<(), CacheError> {
        self.write_dir_with_kind(CacheKind::DirExecuted, payload)
    }

    /// Write a MIR cache entry if enabled.
    pub fn write_mir(&self, payload: ModuleMirData) -> Result<(), CacheError> {
        self.registry.write_mir_cache(
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
            payload,
        )
    }

    /// Read a DIR cache entry for a specific stage if available.
    fn read_dir_with_kind(
        &self,
        cache_kind: CacheKind,
    ) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !self.options.is_enabled() {
            return Ok(None);
        }

        // read from cache registry
        let outcome = self.registry.read_dir_cache_outcome(
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
            cache_kind,
        );

        // record cache outcome
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

        // map cache outcome to response
        match outcome {
            Ok(CacheReadOutcome::Hit { entry, .. }) => Ok(Some(entry)),
            Ok(CacheReadOutcome::Miss) | Ok(CacheReadOutcome::Error) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// Write a DIR cache entry for a specific stage if enabled.
    fn write_dir_with_kind(
        &self,
        cache_kind: CacheKind,
        payload: ModuleDirData,
    ) -> Result<(), CacheError> {
        self.registry.write_dir_cache_with_kind(
            self.cache_store,
            &self.options,
            &self.context,
            self.module_id,
            cache_kind,
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
            | CacheKind::DirResolved
            | CacheKind::DirAnalyzed
            | CacheKind::DirExecuted => match source {
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
