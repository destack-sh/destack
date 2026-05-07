use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{DependencyItem, Export, LocalNodeId, NamespaceExport, StaticKey, SymbolSpace};

/// Resolved module exports for one module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportTable {
    /// Export assignment item when present.
    pub export_assignment: Option<LocalNodeId<DependencyItem>>,
    /// Namespace exports declared in the module.
    pub namespace_exports: Vec<NamespaceExport>,
    /// Export entries by exported symbol key.
    pub export_by_key: IndexMap<(SymbolSpace, StaticKey), Export>,
}

impl ExportTable {
    /// Create an empty export table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return true when the module exposes no exports.
    pub fn is_empty(&self) -> bool {
        self.export_assignment.is_none()
            && self.namespace_exports.is_empty()
            && self.export_by_key.is_empty()
    }
}
