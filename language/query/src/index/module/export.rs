use destack_dir as dir;

use crate::ModuleQueryContext;

/// Builder for one export index from checked DIR.
pub(super) struct ExportIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::ExportEntry>,
}

impl<'context, 'query> ExportIndexer<'context, 'query> {
    /// Build the export index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::ExportIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked export surface
        indexer.collect_exports();

        dir::ExportIndex::new(indexer.entries)
    }

    /// Collect export index entries.
    fn collect_exports(&mut self) {
        let module_path = self.module_path();

        // collect this module's named export rows
        for (key, export) in self.module.exports().exports() {
            let Some(name) = self.export_name(*key) else {
                continue;
            };

            // collect only exports whose target is in this module's checked DIR
            let Some((symbol, source)) = self.export_target(*export) else {
                continue;
            };
            let span = self.module.get_span(self.module.view(), source.local_id);
            let kind = self.export_symbol_kind(symbol);

            self.entries.push(dir::ExportEntry {
                name,
                kind,
                symbol,
                source,
                file: span.file,
                span,
                module_path: module_path.clone(),
            });
        }
    }

    /// Return the source spelling for one export key.
    fn export_name(&self, key: dir::ExportKey) -> Option<String> {
        match key {
            dir::ExportKey::Default => Some("default".to_string()),
            dir::ExportKey::Named(dir::StaticKey::Name(name)) => {
                Some(self.module.strings().get(name).to_string())
            }
            dir::ExportKey::Named(dir::StaticKey::Index(index)) => Some(index.to_string()),
            dir::ExportKey::Named(dir::StaticKey::Symbol(_)) => None,
        }
    }

    /// Return this module's checked target for one named export.
    fn export_target(
        &self,
        export: dir::NamedExport,
    ) -> Option<(dir::GlobalSymbolId, dir::GlobalNodeIdAny)> {
        match export {
            dir::NamedExport::Local(export) => {
                // prefer explicit export item source, otherwise use declaration source
                let symbol = export.source.into_global(self.module.module_id());
                let source = if let Some(item) = export.item {
                    item.into_global_any(self.module.module_id())
                } else {
                    let symbols = self.module.symbols();
                    let exported_symbol = symbols.get_symbol(export.source);
                    exported_symbol.declaration.unwrap_or_else(|| {
                        panic!(
                            "missing declaration for exported symbol {:?}",
                            export.source
                        )
                    })
                };

                Some((symbol, source))
            }
            dir::NamedExport::Indirect(export) => {
                let symbol = self.module.dependency_symbol_target(export.item)?;
                if symbol.module_id != self.module.module_id() {
                    return None;
                }

                let source = export.item.into_global_any(self.module.module_id());

                Some((symbol, source))
            }
        }
    }

    /// Return the symbol kind for an export entry.
    fn export_symbol_kind(&self, symbol_id: dir::GlobalSymbolId) -> dir::SymbolKind {
        // read local symbol kinds from this module's checked DIR
        let symbols = self.module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        symbol.kind
    }

    /// Resolve the importable module path for this module.
    fn module_path(&self) -> Option<String> {
        let repository = self.module.repository();
        let revision = self.module.revision();
        let module_id = self.module.module_id();
        let module = repository
            .module(revision, module_id)
            .unwrap_or_else(|error| panic!("failed to read indexed module {module_id:?}: {error}"))
            .unwrap_or_else(|| panic!("missing indexed module {module_id:?}"));

        // prefer a filesystem path when available
        if let Some(path) = module.path.as_ref() {
            return Some(path.to_string_lossy().to_string());
        }

        // use the module uri
        let uri = module.uri.as_ref();
        let path = if let Some(path) = uri.strip_prefix("file://") {
            path
        } else {
            uri
        };
        if path.is_empty() {
            return None;
        }

        Some(path.to_string())
    }
}
