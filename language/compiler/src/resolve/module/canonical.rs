use crate::timing::tags;
use crate::{Compiler, ResolveError, ResolveResult, TaskResultCollector};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
    /// Compute canonical_symbol for all symbols (phase 2).
    pub(crate) fn resolve_module_canonical(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> ResolveResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::RESOLVE_MODULE_CANONICAL);

        self.require_resolve_module_direct(module_id, profile)?;
        if !self.is_code_module(module_id) {
            self.update_module_graph(module_id, profile, module_version, profile_version)?;
            return Ok(());
        }

        // load module data for canonical resolution
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
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
                        id.into_global(module_id),
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
            drop(module);
            self.update_module_graph(module_id, profile, module_version, profile_version)?;
            return Ok(());
        }

        // drop locks before resolving canonical symbols (may need to access other modules)
        drop(symbols);
        drop(tree);
        drop(module);

        // resolve canonical symbols
        // (this may yield for cross module resolution)
        let mut collector = TaskResultCollector::new();
        for (symbol_id, node) in symbols_to_resolve {
            self.collect(
                &mut collector,
                self.resolve_canonical_symbol(node, symbol_id, profile),
            );
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        self.update_module_graph(module_id, profile, module_version, profile_version)?;
        Ok(())
    }
}
