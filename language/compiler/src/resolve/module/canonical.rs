use destack_dir::SymbolTable;
use destack_workspace::{Module, ProfileId, Revision};

use crate::timing::tags;
use crate::{Compiler, RequirementCollector, ResolveError, ResolveResult};

impl Compiler {
    /// Compute canonical_symbol for all symbols (phase 2).
    pub(crate) fn resolve_module_canonical(
        &self,
        revision: Revision,
        module: &Module,
        profile: ProfileId,
        symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        let _timing = self.timing_scope(tags::RESOLVE_MODULE_CANONICAL);
        if !module.is_code() {
            return Ok(());
        }
        // collect symbols that have target_symbol but no canonical_symbol
        // (only include symbols with primary_declaration, others are internal or incomplete)
        let symbols_to_resolve: Vec<_> = symbols
            .active_symbol_ids()
            .filter_map(|id| {
                let symbol = symbols.get_symbol(id);
                if symbol.target_symbol.is_some()
                    && symbol.canonical_symbol.is_none()
                    && symbol.primary_declaration.is_some()
                {
                    Some((
                        id.into_global(module.id),
                        symbol
                            .primary_declaration
                            .expect("checked primary declaration"),
                    ))
                } else {
                    None
                }
            })
            .collect();

        // update graph when no canonical work is needed
        if symbols_to_resolve.is_empty() {
            return Ok(());
        }
        // resolve canonical symbols
        // (this may yield for cross module resolution)
        let mut collector = RequirementCollector::new();
        for (symbol_id, node) in symbols_to_resolve {
            self.collect(
                &mut collector,
                self.resolve_canonical_symbol(revision, module, symbols, node, symbol_id, profile),
            );
        }

        // yield on any yield
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        Ok(())
    }
}
