use destack_artifact::{DiagnosticAnchor, DirExported};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::{ExportError, ExportResult};

/// Export phase state for one module.
pub(crate) struct ExportState<'a> {
    /// The expanded DIR view.
    pub(in crate::export) view: dir::View<'a>,
    /// The expanded binding table.
    pub(in crate::export) bindings: dir::BindingTable<'static>,
    /// The expanded dependency table.
    pub(in crate::export) dependencies: dir::DependencyTable<'static>,
    /// The module namespace scope.
    pub(in crate::export) namespace_scope: dir::LocalScopeId,
    /// The shared string pool.
    pub(in crate::export) strings: &'a StringPool,
    /// The export table being built.
    pub(in crate::export) exports: dir::ExportTable,
    /// The recoverable diagnostics produced while exporting.
    pub(in crate::export) diagnostics: Vec<ExportError>,
}

impl<'a> ExportState<'a> {
    /// Create export state for one module.
    pub(crate) fn new(
        view: dir::View<'a>,
        namespace_scope: dir::LocalScopeId,
        bindings: dir::BindingTable<'static>,
        dependencies: dir::DependencyTable<'static>,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            view,
            bindings,
            dependencies,
            namespace_scope,
            strings,
            exports: dir::ExportTable::new(view.tree().module_id),
            diagnostics: Vec::new(),
        }
    }

    /// Finish exported DIR.
    pub(in crate::export) fn finish(self) -> (DirExported, Vec<ExportError>) {
        let exported = DirExported {
            exports: self.exports,
        };

        (exported, self.diagnostics)
    }

    /// Insert one named export and report duplicate keys.
    pub(in crate::export) fn insert(
        &mut self,
        export: dir::ExportEntry,
        anchor: DiagnosticAnchor,
    ) -> ExportResult<()> {
        let key = export.key();
        if self.exports.export_by_key.contains_key(&key) {
            self.push_diagnostic(ExportError::DuplicateExport {
                anchor,
                key: self.export_key_text(key),
            });

            return Ok(());
        }

        self.exports.insert(export);

        Ok(())
    }

    /// Push one recoverable export diagnostic.
    pub(in crate::export) fn push_diagnostic(&mut self, diagnostic: ExportError) {
        self.diagnostics.push(diagnostic);
    }

    /// Resolve the target for one re-export expression.
    pub(in crate::export) fn reexport_target(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> ExportResult<Option<ModuleId>> {
        let node_id = expression_id.into_global_any(self.view.tree().module_id);
        self.dependencies
            .edge_for_source(node_id, dir::DependencyRelation::ReExport)
            .map(|edge| edge.target)
            .ok_or_else(|| ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("missing re-export dependency edge for {node_id:?}"),
            })
    }

    /// Find the most recent module-scope symbol for one key.
    pub(in crate::export) fn find_module_symbol(
        &self,
        key: dir::StaticKey,
    ) -> Option<dir::LocalSymbolId> {
        let scope = self.bindings.get_scope_by_id(self.namespace_scope);

        scope.find_symbol(key)
    }

    /// Render one static export key.
    pub(in crate::export) fn static_key_text(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) | dir::StaticKey::Number(name) => {
                self.strings().get(name).to_string()
            }
            dir::StaticKey::Symbol(symbol) => symbol.debug_string(self.strings()),
        }
    }

    /// Render one export key.
    pub(in crate::export) fn export_key_text(&self, key: dir::ExportKey) -> String {
        match key {
            dir::ExportKey::Default => "default".to_string(),
            dir::ExportKey::Named(key) => self.static_key_text(key),
        }
    }

    /// Return the shared string pool.
    pub(in crate::export) fn strings(&self) -> &dir::StringPool {
        self.strings
    }

    /// Return the source anchor for one local node id.
    pub(in crate::export) fn anchor_node(&self, node_id: u32) -> ExportResult<DiagnosticAnchor> {
        self.view
            .get_span_by_id(node_id)
            .map(DiagnosticAnchor::from)
            .ok_or_else(|| ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("missing source span for exported node {node_id}"),
            })
    }

    /// Return the source anchor for one local export.
    pub(in crate::export) fn local_export_anchor(
        &self,
        export: &dir::LocalExportEntry,
    ) -> ExportResult<DiagnosticAnchor> {
        if let Some(item) = export.item {
            return self.anchor_node(item.id);
        }

        let symbol = self.bindings.get_symbol(export.source);
        let Some(declaration) = symbol.declaration else {
            return Err(ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("exported symbol {:?} has no declaration", export.source),
            });
        };

        self.anchor_node(declaration.local_id.id)
    }

    /// Return visible symbol ids.
    pub(in crate::export) fn symbol_ids(&self) -> Vec<dir::LocalSymbolId> {
        self.bindings.symbol_ids().collect()
    }

    /// Check whether one declaration remains visible after expansion.
    pub(in crate::export) fn declaration_is_visible(
        &self,
        declaration: dir::GlobalNodeIdAny,
    ) -> bool {
        if declaration.module_id != self.view.tree().module_id {
            return false;
        }

        self.view.is_visible(declaration.local_id)
    }

    /// Return the module-level diagnostic anchor.
    fn module_anchor(&self) -> DiagnosticAnchor {
        DiagnosticAnchor::from(self.view.tree().module_id)
    }
}
