use crate::{
    AnalyzeError, AnalyzeResult, Compiler, InferContext, InferTable, TaskDependencyError,
    TaskResultCollector,
};
use destack_source::ModuleId;

impl Compiler {
    /// Ensure a module's types have been inferred.
    pub fn require_analyze_module_infer(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleInfer { module })
    }

    /// Phase 2: Infer expression types.
    pub(crate) fn analyze_module_infer(&self, module_id: ModuleId) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let mut collector = TaskResultCollector::new();

        // analyze all expressions
        let mut infer = InferTable::default();
        let mut ctx = InferContext::new();
        for root_id in dir.roots.iter() {
            self.collect(
                &mut collector,
                self.infer_expression(
                    &module,
                    *root_id,
                    &tree,
                    &symbols,
                    &mut types,
                    &mut infer,
                    &mut ctx,
                ),
            );
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        // solve constraints and commit inferred types
        self.solve_infer_table(&infer, &mut types);

        Ok(())
    }
}
