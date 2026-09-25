use smallvec::smallvec;
use tspp_dir as dir;

use crate::export::state::ExportState;
use crate::{Compiler, ExportResult};

impl Compiler {
    /// Collect exports declared by module symbols.
    pub(in crate::export) fn collect_declaration_exports(
        &self,
        state: &mut ExportState<'_>,
    ) -> ExportResult<()> {
        // scan bound symbols
        for symbol_id in state.symbol_ids() {
            state.stats.scanned_symbols += 1;

            self.collect_declaration_global(state, symbol_id)?;

            if let Some(export) = self.declaration_export_entry(state, symbol_id)? {
                let anchor = state.local_export_anchor(&export)?;
                state.insert_export(export, anchor)?;
            }
        }

        Ok(())
    }

    /// Collect one global declaration symbol.
    fn collect_declaration_global(
        &self,
        state: &mut ExportState<'_>,
        symbol_id: dir::LocalSymbolId,
    ) -> ExportResult<()> {
        let symbol = state.bindings.get_symbol(symbol_id);

        // ignore non-global symbols
        if !symbol.origin.is_global() {
            return Ok(());
        }

        // dependency aliases are published by their GlobalEntry
        if symbol.kind == dir::SymbolKind::ExportAlias {
            return Ok(());
        }

        // ignore anonymous and generated globals
        let Some(key) = symbol.key else {
            return Ok(());
        };

        // ignore declarations hidden by expanded patches
        if let Some(declaration) = symbol.declaration {
            if state
                .static_hidden_declarations
                .contains(&declaration.local_id)
            {
                return Ok(());
            }
            if !state.static_allows_owners(declaration.local_id)? {
                return Ok(());
            }
            if !state.declaration_is_visible(declaration) {
                return Ok(());
            }
        }

        let declarations = state.visible_declarations(key);
        if declarations.last() != Some(&symbol_id) {
            return Ok(());
        }

        // retain the complete visible declaration group
        for symbol in &declarations {
            let form = state.symbol_form(*symbol);
            state.exports.insert_symbol_form(*symbol, form);
        }
        state.globals.push(dir::GlobalEntry {
            key,
            item: None,
            declaration: None,
            binding: dir::ExportBinding::Local {
                symbols: declarations,
            },
        });

        Ok(())
    }

    /// Return the export entry declared by one module symbol.
    fn declaration_export_entry(
        &self,
        state: &mut ExportState<'_>,
        symbol_id: dir::LocalSymbolId,
    ) -> ExportResult<Option<dir::NamedExport>> {
        let symbol = state.bindings.get_symbol(symbol_id);
        let scope = symbol.scope.id;
        let declaration = symbol.declaration;
        let export_kind = symbol.export_kind;
        let key = symbol.key;

        // ignore symbols outside the module namespace
        if scope != state.namespace_scope {
            return Ok(None);
        }

        // ignore declarations hidden by expanded patches
        let Some(declaration) = declaration else {
            return Ok(None);
        };
        if state
            .static_hidden_declarations
            .contains(&declaration.local_id)
        {
            return Ok(None);
        }
        if !state.static_allows_owners(declaration.local_id)? {
            return Ok(None);
        }
        if !state.declaration_is_visible(declaration) {
            return Ok(None);
        }

        // resolve visible export name
        let Some(export_kind) = export_kind else {
            return Ok(None);
        };
        let (name, declarations) = match export_kind {
            dir::ExportKind::Default => (dir::ExportKey::default_key(), smallvec![symbol_id]),
            dir::ExportKind::Named => {
                let Some(key) = key else {
                    return Ok(None);
                };
                let declarations = state.visible_declarations(key);
                if declarations.last() != Some(&symbol_id) {
                    return Ok(None);
                }

                (dir::ExportKey::named(key), declarations)
            }
        };

        // retain every exported declaration form
        for symbol in &declarations {
            let form = state.symbol_form(*symbol);
            state.exports.insert_symbol_form(*symbol, form);
        }

        Ok(Some(dir::NamedExport {
            key: name,
            item: None,
            declaration: None,
            binding: dir::ExportBinding::Local {
                symbols: declarations,
            },
        }))
    }
}
