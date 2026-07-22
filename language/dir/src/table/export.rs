use destack_serde::Reflect;
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{ExportForm, ExportKey, LocalSymbolId, NamedExport, StarExport};

/// Resolved module exports for one module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ExportTable {
    /// The module id of the export table.
    pub module_id: ModuleId,
    /// Local and indirect exports keyed by exported name.
    pub export_by_key: IndexMap<ExportKey, NamedExport>,
    /// Star exports declared by the module.
    pub star_exports: Vec<StarExport>,
}

impl ExportTable {
    /// Create an empty export table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            export_by_key: IndexMap::new(),
            star_exports: Vec::new(),
        }
    }

    /// Return true when the module exposes no exports.
    pub fn is_empty(&self) -> bool {
        self.export_by_key.is_empty() && self.star_exports.is_empty()
    }

    /// Insert one named export.
    pub fn insert(&mut self, export: NamedExport) -> Option<NamedExport> {
        self.export_by_key.insert(export.key(), export)
    }

    /// Push one star export.
    pub fn push_star(&mut self, export: StarExport) {
        self.star_exports.push(export);
    }

    /// Iterate named exports in declaration order.
    pub fn exports(&self) -> impl Iterator<Item = (&ExportKey, &NamedExport)> {
        self.export_by_key.iter()
    }

    /// Iterate star exports in declaration order.
    pub fn star_exports(&self) -> impl Iterator<Item = &StarExport> {
        self.star_exports.iter()
    }

    /// Return the declared form behind one locally exported symbol.
    pub fn local_form(&self, symbol: LocalSymbolId) -> Option<ExportForm> {
        self.export_by_key.values().find_map(|export| match export {
            NamedExport::Local(local) if local.source == symbol => Some(local.form),
            _ => None,
        })
    }

    /// Return modules targeted by re-export edges.
    pub fn reexport_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.export_by_key
            .values()
            .filter_map(|export| match export {
                NamedExport::Local(_) => None,
                NamedExport::Indirect(export) => export.target,
            })
            .chain(self.star_exports.iter().filter_map(|export| export.target))
    }
}
