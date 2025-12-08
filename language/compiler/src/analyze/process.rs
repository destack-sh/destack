use crate::{
    AnalyzeError, AnalyzeResult, Compiler, Task, TaskDebug, TaskDependencyError, TaskOutput,
    TaskResultCollector, TypeContext,
};

use destack_dir::LocalTypeId;
use destack_source::ModuleId;
use destack_workspace::Program;

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AnalyzeTask {
    /// Analyze a module.
    AnalyzeModule { module: ModuleId },
}

impl AnalyzeTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::AnalyzeModule { .. } => 1,
        }
    }
}

impl TaskDebug for AnalyzeTask {
    fn name(&self) -> &'static str {
        match self {
            Self::AnalyzeModule { .. } => "analyze module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::AnalyzeModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<AnalyzeTask> for Task {
    fn from(task: AnalyzeTask) -> Self {
        Task::Analyze(task)
    }
}

/// Output of a analyze task.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzeOutput {}

impl From<AnalyzeOutput> for TaskOutput {
    fn from(output: AnalyzeOutput) -> Self {
        TaskOutput::Analyze(output)
    }
}

impl Compiler {
    /// Process an analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<AnalyzeOutput> {
        match task {
            AnalyzeTask::AnalyzeModule { module } => {
                self.require_resolve(module)?;
                self.analyze_module(module)?;
            }
        }
        Ok(AnalyzeOutput {})
    }

    /// Ensure a module has been analyzed.
    pub fn require_analyze(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.require_task(AnalyzeTask::AnalyzeModule { module })
    }

    /// Analyze a module.
    pub fn analyze_module(&self, module_id: ModuleId) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.dir.tree.write();
        let mut symbols = module.dir.symbols.write();
        let mut types = module.dir.types.write();
        let mut collector = TaskResultCollector::new();

        // step 1: evaluate declared types (from annotations)
        let type_count = types.type_count();
        for i in 0..type_count {
            let ty_id = LocalTypeId::new(i);
            self.collect(
                &mut collector,
                self.evaluate_type(&module, ty_id, &mut tree, &mut symbols, &mut types),
            );
        }

        // step 2: analyze types (infer, instantiate, resolve)
        let mut ctx = TypeContext::new();
        for root_id in module.dir.roots.iter() {
            self.collect(
                &mut collector,
                self.analyze_expression(&module, *root_id, &tree, &symbols, &mut types, &mut ctx),
            );
        }

        // step 3: check types (type compatibility, assignability, etc.)
        // TODO #Incomplete: check analyzed types, visibility, overloads, ..

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }
}
