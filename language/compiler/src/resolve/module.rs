use crate::{AnalyzeTask, Compiler, ResolveError, ResolveResult, TaskResultCollector};
use destack_dir::{DependencyItem, Expression, ModuleId};

impl Compiler {
    /// Resolve an entire module lexically.
    pub fn resolve_module(&self, module_id: ModuleId) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.tree.write();
        let mut symbols = module.symbols.write();
        let mut collector = TaskResultCollector::new();

        // resolve expressions (lexical resolution)
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.collect(
                &mut collector,
                self.resolve_expression(&module, expression_id, &mut tree, &mut symbols),
            );
        }

        // resolve dependencies (lexically)
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            self.collect(
                &mut collector,
                self.resolve_dependency_item(&module, item_id, &mut tree, &mut symbols),
            );
        }

        // return combined any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        // next task: analyze module
        self.enqueue(AnalyzeTask::Analyze { module: module_id });

        Ok(())
    }
}
