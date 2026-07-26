use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};

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
    pub(super) fn build(
        module: &'context ModuleQueryContext<'query>,
    ) -> ProviderResult<dir::SymbolIndex> {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked declaration symbols
        indexer.collect_symbols()?;

        Ok(dir::SymbolIndex::new(indexer.entries))
    }

    /// Collect symbol index entries.
    fn collect_symbols(&mut self) -> ProviderResult<()> {
        let module_id = self.module.module_id();

        // collect bound declaration symbols
        for (source, symbol_id) in self.module.symbols().declaration_symbols() {
            // keep only declarations owned by this module
            if source.module_id != module_id {
                continue;
            }

            // index definition members through the dedicated member index
            if matches!(
                source.local_id.ty,
                dir::NodeType::Member | dir::NodeType::TypeMember | dir::NodeType::EnumField
            ) {
                continue;
            }

            // exclude labels and dependency bindings from program declarations
            let symbol = self.module.symbols().get_symbol(symbol_id);
            if matches!(
                symbol.kind,
                dir::SymbolKind::Label | dir::SymbolKind::Import
            ) {
                continue;
            }
            if symbol.role == dir::SymbolRole::Local
                && symbol.scope.id != self.module.namespace_scope()
                && !symbol.origin.is_global()
            {
                continue;
            }

            // skip declarations without a stable queried name
            let Some(name_id) = symbol.name() else {
                continue;
            };

            // emit symbol declaration row
            if let Some(entry) = self.symbol_entry(source, symbol_id, name_id)? {
                self.entries.push(entry);
            }
        }

        Ok(())
    }

    /// Build one symbol index entry.
    fn symbol_entry(
        &self,
        source: dir::GlobalNodeIdAny,
        symbol_id: dir::LocalSymbolId,
        name_id: dir::StringId,
    ) -> ProviderResult<Option<dir::SymbolEntry>> {
        let view = self.module.view();
        let module_id = self.module.module_id();
        let symbol = self.module.symbols().get_symbol(symbol_id);

        // resolve source metadata
        let Some(span) = view.get_span_by_id(source.local_id.id) else {
            return Ok(None);
        };
        let selection = self
            .module
            .node_selection_span(view, source.local_id)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "indexed symbol has no declaration name span: {source:?}"
                ))
            })?;
        let name = self.module.strings().get(name_id).to_string();
        let container = self.module.local_symbol_container_name(symbol_id);
        let global_symbol = symbol_id.into_global(module_id);

        Ok(Some(dir::SymbolEntry {
            name,
            kind: symbol.kind,
            role: symbol.role,
            symbol: global_symbol,
            source,
            file: span.file,
            span,
            selection,
            container,
            mutability: symbol.binding_mutability,
        }))
    }
}
