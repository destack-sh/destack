use crate::{AnalyzeError, AnalyzeResult, Compiler, TaskDependencyError, TaskResultCollector};
use destack_dir::LocalTypeId;
use destack_source::ModuleId;

impl Compiler {
    /// Ensure a module's types have been declared (evaluated).
    pub fn require_analyze_module_declare(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleDeclare { module })
    }

    /// Phase 1: Evaluate declarations.
    pub(crate) fn analyze_module_declare(&self, module_id: ModuleId) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir();
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let mut collector = TaskResultCollector::new();

        // evaluate any unevaluated types
        for i in 0..types.type_count() {
            let ty_id = LocalTypeId::new(i);
            self.collect(
                &mut collector,
                self.evaluate_type(&module, ty_id, &tree, &symbols, &mut types),
            );
        }

        // register extensions
        // 1) register inherent extensions from imported symbols
        // 2) register local extensions from local symbols
        // 3) register named extensions from imported symbols
        // TODO #Incomplete: implement #Extensions

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }
}
