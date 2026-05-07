use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{ImportResolutionKey, ModuleResolution};

/// Resolved module imports for one module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportTable {
    /// Resolved import targets by import key.
    pub resolution_by_key: IndexMap<ImportResolutionKey, ModuleResolution>,
}

impl ImportTable {
    /// Create an empty import table.
    pub fn new() -> Self {
        Self::default()
    }
}
