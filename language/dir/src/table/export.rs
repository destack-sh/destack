use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{ExportEntry, ExportKey, StarExportEntry};

/// Resolved module exports for one module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportTable {
    /// Local and indirect exports keyed by exported name.
    pub export_by_key: IndexMap<ExportKey, ExportEntry>,
    /// Star exports declared by the module.
    pub star_exports: Vec<StarExportEntry>,
}

impl ExportTable {
    /// Create an empty export table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return true when the module exposes no exports.
    pub fn is_empty(&self) -> bool {
        self.export_by_key.is_empty() && self.star_exports.is_empty()
    }

    /// Insert one named export.
    pub fn insert(&mut self, export: ExportEntry) -> Option<ExportEntry> {
        self.export_by_key.insert(export.key(), export)
    }

    /// Push one star export.
    pub fn push_star(&mut self, export: StarExportEntry) {
        self.star_exports.push(export);
    }

    /// Iterate named exports in declaration order.
    pub fn exports(&self) -> impl Iterator<Item = (&ExportKey, &ExportEntry)> {
        self.export_by_key.iter()
    }

    /// Iterate star exports in declaration order.
    pub fn star_exports(&self) -> impl Iterator<Item = &StarExportEntry> {
        self.star_exports.iter()
    }
}
