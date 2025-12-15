use crate::{
    AnalyzeError, AnalyzeResult, Compiler, InferContext, TaskDependencyError, TaskResultCollector,
};
use destack_compiler_macros::DefineTask;
use destack_dir::{Declaration, LocalTypeId, Member, Parameter};
use destack_source::ModuleId;

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Analyze)]
pub enum AnalyzeTask {
    /// Analyze a module completely (declare, infer, validate).
    #[task(code = 1, trace = "module={module}")]
    AnalyzeModule { module: ModuleId },

    /// Analyze declarations.
    #[task(code = 2, trace = "module={module}")]
    AnalyzeModuleDeclare { module: ModuleId },

    /// Infer expression types.
    #[task(code = 3, trace = "module={module}")]
    AnalyzeModuleInfer { module: ModuleId },

    /// Final validation pass.
    #[task(code = 4, trace = "module={module}")]
    AnalyzeModuleValidate { module: ModuleId },
}

impl Compiler {
    /// Process an analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<()> {
        match task {
            AnalyzeTask::AnalyzeModule { module } => {
                self.require_analyze_module_validate(module)?;
            }
            AnalyzeTask::AnalyzeModuleDeclare { module } => {
                self.require_resolve_module_canonical(module)?;
                self.analyze_module_declare(module)?;
            }
            AnalyzeTask::AnalyzeModuleInfer { module } => {
                self.require_analyze_module_declare(module)?;
                self.analyze_module_infer(module)?;
            }
            AnalyzeTask::AnalyzeModuleValidate { module } => {
                self.require_analyze_module_infer(module)?;
                self.analyze_module_validate(module)?;
            }
        }
        Ok(())
    }

    /// Ensure a module's types have been declared (evaluated).
    pub fn require_analyze_module_declare(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleDeclare { module })
    }

    /// Ensure a module's types have been inferred.
    pub fn require_analyze_module_infer(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleInfer { module })
    }

    /// Ensure a module has been validated after analysis.
    pub fn require_analyze_module_validate(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleValidate { module })
    }

    /// Ensure a module has been fully analyzed (including checks).
    pub fn require_analyze_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleValidate { module })
    }

    /// Phase 1: Evaluate declarations.
    fn analyze_module_declare(&self, module_id: ModuleId) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.dir.tree.write();
        let symbols = module.dir.symbols.read();
        let mut types = module.dir.types.write();
        let mut collector = TaskResultCollector::new();

        // evaluate any unevaluated types
        for i in 0..types.type_count() {
            let ty_id = LocalTypeId::new(i);
            self.collect(
                &mut collector,
                self.evaluate_type(&module, ty_id, &mut tree, &symbols, &mut types),
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

    /// Phase 2: Infer expression types.
    fn analyze_module_infer(&self, module_id: ModuleId) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir.tree.read();
        let symbols = module.dir.symbols.read();
        let mut types = module.dir.types.write();
        let mut collector = TaskResultCollector::new();

        // analyze all expressions
        let mut ctx = InferContext::new();
        for root_id in module.dir.roots.iter() {
            self.collect(
                &mut collector,
                self.infer_expression(&module, *root_id, &tree, &symbols, &mut types, &mut ctx),
            );
        }

        // yield on any yields
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }

    /// Phase 3: Final validation checks.
    fn analyze_module_validate(&self, module_id: ModuleId) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir.tree.read();
        let types = module.dir.types.read();

        // validate declarations
        for (id, declaration) in tree.iter_nodes_of_type::<Declaration>() {
            self.validate_declaration(&module, &types, id, declaration);
        }

        // validate parameters
        for (id, parameter) in tree.iter_nodes_of_type::<Parameter>() {
            self.validate_parameter(&module, &tree, id, parameter);
        }

        // validate members
        for (id, member) in tree.iter_nodes_of_type::<Member>() {
            self.validate_member(&module, &tree, id, member);
        }

        Ok(())
    }
}
