use destack_dir as dir;

use crate::ModuleQueryContext;

/// Builder for one symbol index from checked DIR.
pub(super) struct SymbolIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::SymbolEntry>,
}

impl<'context, 'query> SymbolIndexer<'context, 'query> {
    /// Build the symbol index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::SymbolIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked declaration symbols
        indexer.collect_symbols();

        dir::SymbolIndex::new(indexer.entries)
    }

    /// Collect symbol index entries.
    fn collect_symbols(&mut self) {
        let module_id = self.module.module_id();

        // collect bound declaration symbols
        for (source, symbol_id) in self.module.symbols().declaration_symbols() {
            // keep only declarations owned by this module
            if source.module_id != module_id {
                continue;
            }

            // skip labels because they are not query symbols
            let symbol = self.module.symbols().get_symbol(symbol_id);
            if symbol.kind == dir::SymbolKind::Label {
                continue;
            }

            // skip declarations without a stable queried name
            let Some(name_id) = symbol.name() else {
                continue;
            };

            // emit symbol declaration row
            let entry = self.symbol_entry(source, symbol_id, name_id);
            self.entries.push(entry);
        }
    }

    /// Build one symbol index entry.
    fn symbol_entry(
        &self,
        source: dir::GlobalNodeIdAny,
        symbol_id: dir::LocalSymbolId,
        name_id: dir::StringId,
    ) -> dir::SymbolEntry {
        let view = self.module.view();
        let module_id = self.module.module_id();
        let symbol = self.module.symbols().get_symbol(symbol_id);

        // resolve source metadata
        let span = self.module.get_span(view, source.local_id);
        let name = self.module.strings().get(name_id).to_string();
        let container = self.module.node_container_name(source.local_id);

        // resolve checked type metadata
        let global_symbol = symbol_id.into_global(module_id);
        let ty = self.module.types().get_symbol_type_id(global_symbol);

        dir::SymbolEntry {
            name,
            kind: symbol.kind,
            role: symbol.role,
            symbol: global_symbol,
            source,
            file: self.module.file_id(),
            span,
            ty,
            container,
            mutability: symbol.binding_mutability,
            is_exported: symbol.export_kind.is_some(),
        }
    }
}
