use destack_source::{CacheKind, ModuleId};
use destack_workspace::{
    CacheError, ModuleAstCacheEntry, ModuleAstData, ModuleDirCacheEntry, ModuleDirData,
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
        let outcome =
            self.registry
                .read_ast_cache_outcome(&self.options, &self.context, self.module_id);

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

    /// Read a DIR cache entry if available.
    pub fn read_dir(&self) -> Result<Option<ModuleDirCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !self.options.is_enabled() {
            return Ok(None);
        }

        // read from cache registry
        let outcome =
            self.registry
                .read_dir_cache_outcome(&self.options, &self.context, self.module_id);

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

    /// Read a MIR cache entry if available.
    pub fn read_mir(&self) -> Result<Option<ModuleMirCacheEntry>, CacheError> {
        // skip when cache is disabled
        if !self.options.is_enabled() {
            return Ok(None);
        }

        // read from cache registry
        let outcome =
            self.registry
                .read_mir_cache_outcome(&self.options, &self.context, self.module_id);

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
        self.registry
            .write_ast_cache(&self.options, &self.context, self.module_id, payload)
    }

    /// Write a DIR cache entry if enabled.
    pub fn write_dir(&self, payload: ModuleDirData) -> Result<(), CacheError> {
        self.registry
            .write_dir_cache(&self.options, &self.context, self.module_id, payload)
    }

    /// Write a MIR cache entry if enabled.
    pub fn write_mir(&self, payload: ModuleMirData) -> Result<(), CacheError> {
        self.registry
            .write_mir_cache(&self.options, &self.context, self.module_id, payload)
    }

    /// Record cache hit statistics for a cache kind.
    fn record_cache_hit(&self, source: CacheReadSource, cache_kind: CacheKind) {
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
