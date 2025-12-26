use crate::{Compiler, ResolveError, ResolveResult, TaskResultCollector};
use destack_dir::{Declaration, DependencyItem, Expression, LocalSymbolId};

use destack_source::ModuleId;
use destack_workspace::{ModuleDir, ProfileId};

impl Compiler {
    /// Resolve expressions, dependencies, and declarations (phase 1).
    pub(super) fn resolve_module_direct(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);

        // initialize DIR for this profile
        {
            let mut module = module.write();
            if module.dirs.iter().all(|dir| dir.profile_id != profile) {
                let dir = {
                    let base = module.dir_base.as_ref().expect("no base DIR on {module:?}");
                    ModuleDir::from_base(profile, base)
                };
                module.dirs.push(dir);
            }
        }

        let module = module.read();
        let dir = module.dir(profile);
        let mut tree = dir.tree.write();
        let mut symbols = dir.symbols.write();
        let mut collector = TaskResultCollector::new();

        // resolve expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.collect(
                &mut collector,
                self.resolve_expression(
                    &module,
                    dir,
                    profile,
                    expression_id,
                    &mut tree,
                    &mut symbols,
                ),
            );
        }

        // resolve dependencies
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            self.collect(
                &mut collector,
                self.resolve_dependency_item(
                    &module,
                    dir,
                    profile,
                    item_id,
                    &mut tree,
                    &mut symbols,
                ),
            );
        }

        // resolve declarations (e.g., extension target_symbol)
        for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
            self.collect(
                &mut collector,
                self.resolve_declaration(&module, dir, declaration_id, &mut tree, &mut symbols),
            );
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        Ok(())
    }

    /// Compute canonical_symbol for all symbols (phase 2).
    pub(super) fn resolve_module_canonical(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // collect symbols that have target_symbol but no canonical_symbol
        // (only include symbols with primary_declaration, others are internal/incomplete)
        let symbols_to_resolve: Vec<_> = (0..symbols.symbol_count())
            .map(LocalSymbolId::new)
            .filter_map(|id| {
                let symbol = symbols.get_symbol(id);
                if symbol.target_symbol.is_some()
                    && symbol.canonical_symbol.is_none()
                    && symbol.primary_declaration.is_some()
                {
                    Some((
                        id.into_global(module_id),
                        symbol.primary_declaration.unwrap(),
                    ))
                } else {
                    None
                }
            })
            .collect();

        // drop locks before resolving canonical symbols (may need to access other modules)
        drop(symbols);
        drop(tree);
        drop(module);

        // resolve canonical symbols (may yield for cross-module resolution)
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

        Ok(())
    }
}
