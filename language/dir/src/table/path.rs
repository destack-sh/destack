use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, GlobalSymbolId};

/// Resolved source paths for one module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathTable {
    /// The module id of the path table.
    pub module_id: ModuleId,
    /// Resolved paths keyed by source path and prefix length.
    pub entries: IndexMap<PathKey, PathResolution>,
}

impl PathTable {
    /// Create an empty path table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            entries: IndexMap::new(),
        }
    }

    /// Insert one resolved path prefix.
    pub fn insert(&mut self, key: PathKey, resolution: PathResolution) {
        self.entries.insert(key, resolution);
    }

    /// Return one resolved path prefix.
    pub fn get(&self, key: PathKey) -> Option<&PathResolution> {
        self.entries.get(&key)
    }

    /// Return true when no paths were resolved.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Source path identity plus resolved prefix length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathKey {
    /// The source node that owns the path syntax.
    pub source: GlobalNodeIdAny,
    /// The number of path segments covered by this resolution.
    pub length: u32,
}

impl PathKey {
    /// Create a path key.
    pub fn new(source: GlobalNodeIdAny, length: u32) -> Self {
        Self { source, length }
    }
}

/// A resolved source path target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathTarget {
    /// The path resolves to a symbol.
    Symbol(GlobalSymbolId),
    /// The path resolves to a namespace module.
    Namespace(ModuleId),
}

/// A resolved source path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathResolution {
    /// The path resolves to one target.
    Found(PathTarget),
    /// The path has no target.
    Missing,
    /// The path has multiple possible targets.
    Ambiguous(Vec<PathTarget>),
}
