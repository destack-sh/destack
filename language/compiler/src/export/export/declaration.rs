use destack_dir as dir;

use crate::export::state::ExportState;
use crate::{Compiler, ExportResult};

impl Compiler {
    /// Collect module surfaces declared by symbols.
    pub(in crate::export) fn collect_declaration_exports(
        &self,
        state: &mut ExportState<'_>,
    ) -> ExportResult<()> {
        // scan bound symbols
        for symbol_id in state.symbol_ids() {
            self.collect_declaration_global(state, symbol_id);

            if let Some(export) = self.declaration_export_entry(state, symbol_id) {
                let anchor = state.local_export_anchor(&export)?;
                state.insert_export(dir::ExportEntry::Local(export), anchor)?;
            }
        }

        Ok(())
    }

    /// Collect one global declaration symbol.
    fn collect_declaration_global(
        &self,
        state: &mut ExportState<'_>,
        symbol_id: dir::LocalSymbolId,
    ) {
        let symbol = state.bindings.get_symbol(symbol_id);

        // ignore non-global symbols
        if !symbol.origin.is_global() {
            return;
        }

        // ignore anonymous and generated globals
        let Some(key) = symbol.key else {
            return;
        };

        // ignore declarations hidden by expanded patches
        if let Some(declaration) = symbol.declaration {
            if !state.declaration_is_visible(declaration) {
                return;
            }
        }

        state.globals.push_local(key, symbol_id);
    }

    /// Return the export entry declared by one module symbol.
    fn declaration_export_entry(
        &self,
        state: &ExportState<'_>,
        symbol_id: dir::LocalSymbolId,
    ) -> Option<dir::LocalExportEntry> {
        let symbol = state.bindings.get_symbol(symbol_id);

        // ignore symbols outside the module namespace
        if symbol.scope.id != state.namespace_scope {
            return None;
        }

        // ignore declarations hidden by expanded patches
        let declaration = symbol.declaration?;
        if !state.declaration_is_visible(declaration) {
            return None;
        }

        // resolve visible export name
        let name = match symbol.export_kind? {
            dir::ExportKind::Default => dir::ExportKey::default_key(),
            dir::ExportKind::Named => dir::ExportKey::named(symbol.key?),
        };

        Some(dir::LocalExportEntry {
            key: name,
            source: symbol_id,
            item: None,
        })
    }
}
