use serde::{Deserialize, Serialize};
use tspp_core::FxIndexMap as IndexMap;
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{ExportBinding, ExportForm, ExportKey, LocalSymbolId, NamedExport, StarExport};

/// Resolved module exports for one module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ExportTable {
    /// The module id of the export table.
    pub module_id: ModuleId,
    /// Local and indirect exports keyed by exported name.
    pub export_by_key: IndexMap<ExportKey, NamedExport>,
    /// Forms of local symbols visible across modules.
    pub form_by_symbol: IndexMap<LocalSymbolId, ExportForm>,
    /// Star exports declared by the module.
    pub star_exports: Vec<StarExport>,
}

impl ExportTable {
    /// Create an empty export table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            export_by_key: IndexMap::default(),
            form_by_symbol: IndexMap::default(),
            star_exports: Vec::new(),
        }
    }

    /// Return true when the module exposes no named or star exports.
    pub fn is_empty(&self) -> bool {
        self.export_by_key.is_empty() && self.star_exports.is_empty()
    }

    /// Insert one named export.
    pub fn insert(&mut self, export: NamedExport) -> Option<NamedExport> {
        self.export_by_key.insert(export.key(), export)
    }

    /// Record the form of one local symbol visible across modules.
    pub fn insert_symbol_form(&mut self, symbol: LocalSymbolId, form: ExportForm) {
        self.form_by_symbol.insert(symbol, form);
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

    /// Return the form of one local symbol visible across modules.
    pub fn symbol_form(&self, symbol: LocalSymbolId) -> Option<ExportForm> {
        self.form_by_symbol.get(&symbol).copied()
    }

    /// Return modules targeted by re-export edges.
    pub fn reexport_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.export_by_key
            .values()
            .filter_map(|export| match export.binding {
                ExportBinding::Local { .. } => None,
                ExportBinding::Import { module, .. } | ExportBinding::ReExport { module, .. } => {
                    module
                }
            })
            .chain(self.star_exports.iter().filter_map(|export| export.target))
    }
}
