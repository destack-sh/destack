use crate::{AnalyzeTask, Compiler, ResolveError, ResolveResult, TaskResultCollector};
use destack_dir::{DependencyItem, Expression};

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

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        // next task: analyze module
        self.enqueue(AnalyzeTask::Analyze { module: module_id });

        Ok(())
    }
}
