use indexmap::{IndexMap, IndexSet};
use smallvec::{SmallVec, smallvec};
use tspp_artifact::{DiagnosticAnchor, DiagnosticBuilder, DirExported, ProfileKey};
use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_repository::{Environment, Module, Package};
use tspp_source::ModuleId;

use crate::export::stats::ExportStats;
use crate::{ExportError, ExportResult};

/// Export phase state for one module.
pub(crate) struct ExportState<'a> {
    /// The expanded DIR view.
    pub(in crate::export) view: dir::View<'a>,
    /// The current module.
    pub(in crate::export) module: &'a Module,
    /// The package containing the current module.
    pub(in crate::export) package: &'a Package,
    /// The ambient environment captured by the current revision.
    pub(in crate::export) environment: &'a Environment,
    /// The active profile key.
    pub(in crate::export) profile: &'a ProfileKey,
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
    /// The diagnostics produced while exporting.
    pub(in crate::export) diagnostics: Vec<DiagnosticBuilder<ExportError>>,
    /// The work stats accumulated while exporting.
    pub(in crate::export) stats: ExportStats,
}

impl<'a> ExportState<'a> {
    /// Create export state for one module.
    pub(crate) fn new(
        view: dir::View<'a>,
        module: &'a Module,
        package: &'a Package,
        environment: &'a Environment,
        profile: &'a ProfileKey,
        namespace_scope: dir::LocalScopeId,
        bindings: dir::BindingTable<'static>,
        modules: dir::ModuleTable<'static>,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            view,
            module,
            package,
            environment,
            profile,
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
    pub(in crate::export) fn finish(self) -> (DirExported, Vec<DiagnosticBuilder<ExportError>>) {
        let locals = self.module_scope_names();
        let exported = DirExported {
            exports: self.exports,
            globals: self.globals,
            locals,
        };

        (exported, self.diagnostics)
    }

    /// Collect the module-scope binding names in declaration order.
    pub(in crate::export) fn module_scope_names(&self) -> Vec<String> {
        let scope = self.bindings.module_scope();
        let mut names = Vec::new();
        for (key, _symbol) in self
            .bindings
            .get_scope(scope)
            .named_symbols_up_to(scope.mark)
        {
            if let dir::StaticKey::Name(name) = key {
                names.push(self.strings.get(name).to_string());
            }
        }
        names.dedup();

        names
    }

    /// Insert one named export and report duplicate keys.
    pub(in crate::export) fn insert_export(
        &mut self,
        export: dir::NamedExport,
        anchor: DiagnosticAnchor,
    ) -> ExportResult<()> {
        let key = export.key();
        if let Some(previous) = self.exports.export_by_key.get(&key) {
            // report a repeated exported name
            let previous_item = previous.item();
            let error = ExportError::DuplicateExport {
                anchor,
                key: self.export_key_text(key),
            };
            let diagnostic = match previous_item.map(|item| self.anchor_node(item.id)) {
                Some(Ok(first)) => {
                    DiagnosticBuilder::new(error).label(first, "first exported here")
                }
                _ => DiagnosticBuilder::new(error),
            };
            self.report_diagnostic(diagnostic);

            return Ok(());
        }

        self.exports.insert(export);

        Ok(())
    }

    /// Report one export diagnostic.
    pub(in crate::export) fn report_diagnostic(
        &mut self,
        diagnostic: impl Into<DiagnosticBuilder<ExportError>>,
    ) {
        self.diagnostics.push(diagnostic.into());
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

    /// Return the visible module declaration group for one key.
    pub(in crate::export) fn visible_declarations(
        &self,
        key: dir::StaticKey,
    ) -> SmallVec<[dir::LocalSymbolId; 2]> {
        let scope = self.bindings.get_scope_by_id(self.namespace_scope);
        let scope = dir::LocalScope::new(self.namespace_scope, scope.mark());
        let lookup = self.bindings.lookup_symbol_from_scope(scope, key);
        let mut candidates = match lookup {
            dir::SymbolLookup::Missing => SmallVec::new(),
            dir::SymbolLookup::Found(symbol) => smallvec![symbol],
            dir::SymbolLookup::Ambiguous(symbols) => symbols.into_iter().collect(),
        };

        // preserve a complete function overload group
        let Some(last) = candidates.last().copied() else {
            return SmallVec::new();
        };
        if self.bindings.get_symbol(last).kind == dir::SymbolKind::Function {
            candidates.retain(|symbol| {
                self.bindings.get_symbol(*symbol).kind == dir::SymbolKind::Function
            });

            return candidates;
        }

        smallvec![last]
    }

    /// Return the explicit declaration introduced by one export item.
    pub(in crate::export) fn export_declaration(
        &self,
        item: dir::LocalNodeId<dir::DependencyItem>,
    ) -> ExportResult<Option<dir::LocalSymbolId>> {
        if self.view.get(item).alias().is_none() {
            return Ok(None);
        }

        // require the binding declared for the authored alias
        let anchor = self.anchor_node(item.id)?;
        let declaration = item.into_global_any(self.view.tree().module_id);
        let symbol = self
            .bindings
            .declaration_symbol(declaration)
            .ok_or_else(|| ExportError::Internal {
                anchor: anchor.clone(),
                module: self.view.tree().module_id,
                message: format!("export alias {item:?} has no declaration"),
            })?;
        if self.bindings.get_symbol(symbol).kind != dir::SymbolKind::ExportAlias {
            return Err(ExportError::Internal {
                anchor,
                module: self.view.tree().module_id,
                message: format!("export alias {item:?} has invalid symbol {symbol:?}"),
            });
        }

        Ok(Some(symbol))
    }

    /// Return the imported route selected by one local symbol.
    pub(in crate::export) fn import_binding(
        &self,
        symbol_id: dir::LocalSymbolId,
    ) -> ExportResult<Option<dir::ExportBinding>> {
        let symbol = self.bindings.get_symbol(symbol_id);
        if symbol.kind != dir::SymbolKind::Import {
            return Ok(None);
        }

        // require a local import declaration
        let declaration = symbol.declaration.ok_or_else(|| ExportError::Internal {
            anchor: self.module_anchor(),
            module: self.view.tree().module_id,
            message: format!("import symbol {symbol_id:?} has no declaration"),
        })?;
        if declaration.module_id != self.view.tree().module_id {
            return Err(ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("import symbol {symbol_id:?} belongs to another module"),
            });
        }

        // read the selected imported name
        let item_id = declaration
            .local_id
            .try_into_typed::<dir::DependencyItem>()
            .map_err(|_| ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("import symbol {symbol_id:?} has invalid declaration"),
            })?;
        let selector =
            self.view
                .get(item_id)
                .export_selector()
                .ok_or_else(|| ExportError::Internal {
                    anchor: self.module_anchor(),
                    module: self.view.tree().module_id,
                    message: format!("import item {item_id:?} has no selector"),
                })?;

        // read the exact import edge owned by the parent expression
        let expression_id = self
            .view
            .get_parent_for(item_id)
            .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
            .ok_or_else(|| ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("import item {item_id:?} has no expression owner"),
            })?;
        let source = expression_id.into_global_any(self.view.tree().module_id);
        let edge = self
            .modules
            .edge_for_source(source, dir::ModuleRelation::Import)
            .ok_or_else(|| ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("import expression {expression_id:?} has no module edge"),
            })?;

        Ok(Some(dir::ExportBinding::Import {
            local: symbol_id,
            module: edge.target,
            selector,
        }))
    }

    /// Render one static export key.
    pub(in crate::export) fn static_key_text(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.strings().get(name).to_string(),
            dir::StaticKey::Index(index) => index.to_string(),
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
        export: &dir::NamedExport,
    ) -> ExportResult<DiagnosticAnchor> {
        if let Some(item) = export.item {
            return self.anchor_node(item.id);
        }

        let dir::ExportBinding::Local { symbols } = &export.binding else {
            return Err(ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("indirect export {:?} has no local anchor", export.key),
            });
        };
        let Some(source) = symbols.first().copied() else {
            return Err(ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("local export {:?} has no source", export.key),
            });
        };
        let symbol = self.bindings.get_symbol(source);
        let Some(declaration) = symbol.declaration else {
            return Err(ExportError::Internal {
                anchor: self.module_anchor(),
                module: self.view.tree().module_id,
                message: format!("exported symbol {source:?} has no declaration"),
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
