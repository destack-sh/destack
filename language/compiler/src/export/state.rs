use destack_artifact::{ConditionSet, DiagnosticAnchor, DirExported, ProfileKey};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::Module;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};

use crate::export::stats::ExportStats;
use crate::{ExportError, ExportResult};

/// Export phase state for one module.
pub(crate) struct ExportState<'a> {
    /// The expanded DIR view.
    pub(in crate::export) view: dir::View<'a>,
    /// The current module.
    pub(in crate::export) module: &'a Module,
    /// The active profile key.
    pub(in crate::export) profile: &'a ProfileKey,
    /// The active profile conditions.
    pub(in crate::export) conditions: &'a ConditionSet,
    /// The expanded binding table.
    pub(in crate::export) bindings: dir::BindingTable<'static>,
    /// The expanded module table.
    pub(in crate::export) modules: dir::ModuleTable<'static>,
    /// The module namespace scope.
    pub(in crate::export) namespace_scope: dir::LocalScopeId,
    /// The shared string pool.
    pub(in crate::export) strings: &'a StringPool,
    /// The export table being built.
    pub(in crate::export) exports: dir::ExportTable,
    /// The global table being built.
    pub(in crate::export) globals: dir::GlobalTable,
    /// Declarations hidden by static guards.
    pub(in crate::export) static_hidden_declarations: IndexSet<dir::LocalNodeIdAny>,
    /// Static visibility decisions by checked node.
    pub(in crate::export) static_visibility_by_node: IndexMap<dir::LocalNodeIdAny, bool>,
    /// Nodes skipped by static guards.
    pub(in crate::export) static_skipped_nodes: IndexSet<dir::LocalNodeIdAny>,
    /// The recoverable diagnostics produced while exporting.
    pub(in crate::export) diagnostics: Vec<ExportError>,
    /// The work stats accumulated while exporting.
    pub(in crate::export) stats: ExportStats,
}

impl<'a> ExportState<'a> {
    /// Create export state for one module.
    pub(crate) fn new(
        view: dir::View<'a>,
        module: &'a Module,
        profile: &'a ProfileKey,
        conditions: &'a ConditionSet,
        namespace_scope: dir::LocalScopeId,
        bindings: dir::BindingTable<'static>,
        modules: dir::ModuleTable<'static>,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            view,
            module,
            profile,
            conditions,
            bindings,
            modules,
            namespace_scope,
            strings,
            exports: dir::ExportTable::new(view.tree().module_id),
            globals: dir::GlobalTable::new(view.tree().module_id),
            static_hidden_declarations: IndexSet::new(),
            static_visibility_by_node: IndexMap::new(),
            static_skipped_nodes: IndexSet::new(),
            diagnostics: Vec::new(),
            stats: ExportStats::default(),
        }
    }

    /// Finish exported DIR.
    pub(in crate::export) fn finish(self) -> (DirExported, Vec<ExportError>) {
        let exported = DirExported {
            exports: self.exports,
            globals: self.globals,
        };

        (exported, self.diagnostics)
    }

    /// Insert one named export and report duplicate keys.
    pub(in crate::export) fn insert_export(
        &mut self,
        export: dir::ExportEntry,
        anchor: DiagnosticAnchor,
    ) -> ExportResult<()> {
        let key = export.key();
        if self.exports.export_by_key.contains_key(&key) {
            self.report_diagnostic(ExportError::DuplicateExport {
                anchor,
                key: self.export_key_text(key),
            });

            return Ok(());
        }

        self.exports.insert(export);

        Ok(())
    }

    /// Report one recoverable export diagnostic.
    pub(in crate::export) fn report_diagnostic(&mut self, diagnostic: ExportError) {
        self.diagnostics.push(diagnostic);
    }

    /// Mark one node as skipped by a static guard.
    pub(in crate::export) fn skip_static_node(&mut self, node: dir::LocalNodeIdAny) {
        if self.static_skipped_nodes.insert(node) {
            self.stats.skipped += 1;
        }
    }

    /// Resolve the target for one re-export expression.
    pub(in crate::export) fn reexport_target(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> ExportResult<Option<ModuleId>> {
        let node_id = expression_id.into_global_any(self.view.tree().module_id);
        self.modules
            .edge_for_source(node_id, dir::ModuleRelation::ReExport)
            .map(|edge| edge.target)
            .ok_or_else(|| ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("missing re-export module edge for {node_id:?}"),
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

    /// Return the target module for one namespace import symbol.
    pub(in crate::export) fn namespace_import_target(
        &self,
        symbol_id: dir::LocalSymbolId,
    ) -> Option<ModuleId> {
        // require an import symbol
        let symbol = self.bindings.get_symbol(symbol_id);
        if symbol.kind != dir::SymbolKind::Import {
            return None;
        }

        // require a local dependency item declaration
        let declaration = symbol.declaration?;
        if declaration.module_id != self.view.tree().module_id {
            return None;
        }

        // require a namespace import item
        let item_id = declaration
            .local_id
            .try_into_typed::<dir::DependencyItem>()
            .ok()?;
        let item = self.view.get(item_id);
        if item.binding()? != dir::DependencyBinding::Namespace {
            return None;
        }

        // find the import edge that owns this item
        for edge in self.modules.iter() {
            if edge.relation != dir::ModuleRelation::Import {
                continue;
            }

            let Ok(expression_id) = edge.source.local_id.try_into_typed::<dir::Expression>() else {
                continue;
            };
            let expression = self.view.get(expression_id);
            let dir::Expression::Import {
                items: Some(items), ..
            } = expression
            else {
                continue;
            };
            if items.contains(&item_id) {
                return edge.target;
            }
        }

        None
    }

    /// Render one static export key.
    pub(in crate::export) fn static_key_text(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.strings().get(name).to_string(),
            dir::StaticKey::Index(index) => index.to_string(),
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
