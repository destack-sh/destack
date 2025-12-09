use crate::{Compiler, ResolveError, ResolveResult, TaskResultCollector};
use destack_dir::{Declaration, DependencyItem, Expression, LocalSymbolId};

use destack_source::ModuleId;

impl Compiler {
    /// Resolve an entire module lexically.
    pub(super) fn resolve_module(&self, module_id: ModuleId) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.dir.tree.write();
        let mut symbols = module.dir.symbols.write();
        let mut collector = TaskResultCollector::new();

        // resolve expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.collect(
                &mut collector,
                self.resolve_expression(&module, expression_id, &mut tree, &mut symbols),
            );
        }

        // resolve dependencies
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            self.collect(
                &mut collector,
                self.resolve_dependency_item(&module, item_id, &mut tree, &mut symbols),
            );
        }

        // resolve declarations (e.g., extension target_symbol)
        for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
            self.collect(
                &mut collector,
                self.resolve_declaration(&module, declaration_id, &mut tree, &mut symbols),
            );
        }

        // yield on any yield before final symbol resolution
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        // collect symbols that have target_symbol but no final_symbol
        // (only include symbols with primary_declaration, others are internal/incomplete)
        let symbols_to_resolve: Vec<_> = (0..symbols.symbol_count())
            .map(LocalSymbolId::new)
            .filter_map(|id| {
                let symbol = symbols.get_symbol(id);
                if symbol.target_symbol.is_some()
                    && symbol.final_symbol.is_none()
                    && symbol.primary_declaration.is_some()
                {
                    Some((id.into_global(module_id), symbol.primary_declaration.unwrap()))
                } else {
                    None
                }
            })
            .collect();

        // drop locks before resolving final symbols (may need to access other modules)
        drop(symbols);
        drop(tree);
        drop(module);

        // resolve final symbols (may yield for cross-module resolution)
        let mut collector = TaskResultCollector::new();
        for (symbol_id, node) in symbols_to_resolve {
            self.collect(&mut collector, self.resolve_final_symbol(node, symbol_id));
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        Ok(())
    }
}
