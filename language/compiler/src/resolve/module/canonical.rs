use crate::timing::tags;
use crate::{
    BuildRequirementCollector, Compiler, ResolveError, ResolveModuleContext, ResolveResult,
};
use destack_workspace::{ModuleDir, ProfileId};

impl Compiler {
    /// Compute canonical_symbol for all symbols (phase 2).
    pub(crate) fn resolve_module_canonical(
        &self,
        module: &ResolveModuleContext,
        profile: ProfileId,
        dir: &ModuleDir,
    ) -> ResolveResult<()> {
        let _timing = self.timing_scope(tags::RESOLVE_MODULE_CANONICAL);
        if !self.is_code_module(module.id) {
            return Ok(());
        }
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

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
            drop(symbols);
            drop(tree);
            return Ok(());
        }

        // drop locks before resolving canonical symbols (may need to access other modules)
        drop(symbols);
        drop(tree);
        // resolve canonical symbols
        // (this may yield for cross module resolution)
        let mut collector = BuildRequirementCollector::new();
        for (symbol_id, node) in symbols_to_resolve {
            self.collect(
                &mut collector,
                self.resolve_canonical_symbol(module, dir, node, symbol_id, profile),
            );
        }

        // yield on any yield
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        Ok(())
    }
}
