use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dyst_source::FileId;
use serde::Deserialize;

/// Unique identifier for DsConfigs.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DsConfigId(pub u32);

impl DsConfigId {
    /// Wrap an id as a DsConfigId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Dyst configuration (usually from `dsconfig.json`).
#[derive(Debug, Clone)]
pub struct DsConfig {
    /// The id of the DsConfig.
    pub id: DsConfigId,
    /// The id of the `dsconfig.json` file.
    pub file_id: FileId,
    /// Whether this is the root dsconfig in its context.
    pub is_root: bool,
    /// Path to the `dsconfig.json` file (including the `dsconfig.json`).
    pub path: PathBuf,
    /// The directory containing the `dsconfig.json` file.
    pub directory: PathBuf,
    /// The content of the `dsconfig.json` file.
    pub content: DsConfigJson,
}

/// DsConfig JSON (usually from `dsconfig.json`)
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DsConfigJson {
    /// Extends other dsconfigs.
    pub extends: Option<DsConfigExtendsField>,
    /// Compiler options.
    pub compiler_options: DsConfigCompilerOptionsJson,
    /// Targets.
    pub targets: Option<IndexMap<String, DsConfigTargetJson>>,
}

impl DsConfig {}

/// Value for the "extends" field of a dsconfig.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum DsConfigExtendsField {
    /// Extend a single dsconfig.
    Single(String),
    /// Extend multiple dsconfigs.
    Multiple(Vec<String>),
}

/// Dyst configuration compiler options.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DsConfigCompilerOptionsJson {}

/// Dyst target.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DsConfigTargetJson {
    /// whether this is a debug build.
    pub debug: bool,
    /// whether this is an optimized build.
    pub optimize: bool,
    /// Optimization level (0-3).
    pub optimize_level: Option<u8>,
    /// Shrink levels (0-3).
    pub shrink_level: Option<u8>,
}

/// Dyst configuration registry.
#[derive(Debug)]
pub struct DsConfigRegistry {
    /// The dsconfigs by id.
    dsconfigs_by_id: Mutex<HashMap<DsConfigId, Arc<RwLock<DsConfig>>>>,
    /// The next dsconfig id.
    next_dsconfig_id: AtomicU32,
}

impl Default for DsConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DsConfigRegistry {
    /// Create a new DsConfigRegistry.
    pub fn new() -> Self {
        Self {
            dsconfigs_by_id: Mutex::new(HashMap::new()),
            next_dsconfig_id: AtomicU32::new(0),
        }
    }

    /// Get and increment the next dsconfig id.
    pub fn next_id(&self) -> DsConfigId {
        let next_dsconfig_id = self.next_dsconfig_id.fetch_add(1, Ordering::Relaxed);
        DsConfigId::new(next_dsconfig_id)
    }

    /// Insert a dsconfig into the registry.
    pub fn insert(&self, dsconfig: DsConfig) {
        let mut dsconfigs_by_id = self.dsconfigs_by_id.lock();
        dsconfigs_by_id.insert(dsconfig.id, Arc::new(RwLock::new(dsconfig)));
    }

    /// Get a dsconfig by id.
    ///
    /// # Panics
    /// Panics if the dsconfig is not found.
    pub fn get(&self, id: DsConfigId) -> Arc<RwLock<DsConfig>> {
        let dsconfigs_by_id = self.dsconfigs_by_id.lock();
        dsconfigs_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("dsconfig not found: {id:?}"))
            .clone()
    }

    /// Iterate over the dsconfigs in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<DsConfig>>> {
        let dsconfigs_by_id = self.dsconfigs_by_id.lock();
        let snapshot: Vec<_> = dsconfigs_by_id.values().cloned().collect();
        snapshot.into_iter()
    }

    /// Get the number of dsconfigs in the registry.
    pub fn len(&self) -> usize {
        let dsconfigs_by_id = self.dsconfigs_by_id.lock();
        dsconfigs_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        let dsconfigs_by_id = self.dsconfigs_by_id.lock();
        dsconfigs_by_id.is_empty()
    }
}
