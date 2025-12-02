use crate::{AnalyzeError, AnalyzeResult, Compiler, Task, TaskDebug, TaskOutput, TaskResultCollector};

use destack_dir::{LocalTypeId, ModuleId, Program};

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AnalyzeTask {
    /// Analyze a module.
    Analyze { module: ModuleId },
}

impl AnalyzeTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Analyze { .. } => 1,
        }
    }
}

impl TaskDebug for AnalyzeTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Analyze { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Analyze { module } => {
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
    /// Process a analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<AnalyzeOutput> {
        match task {
            AnalyzeTask::Analyze { module } => self.analyze_module(module)?,
        }
        Ok(AnalyzeOutput {})
    }

    /// Analyze a module.
    pub fn analyze_module(&self, module_id: ModuleId) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.tree.write();
        let mut symbols = module.symbols.write();
        let mut types = module.types.write();
        let mut collector = TaskResultCollector::new();

        // step 1: evaluate declared types
        let type_count = types.type_count();
        for i in 0..type_count {
            let ty_id = LocalTypeId::new(i);
            self.collect(
                &mut collector,
                self.evaluate_type(&module, ty_id, &mut tree, &mut symbols, &mut types),
            );
        }

        // step 2: infer types, instances & resolutions
        // ...

        // step 3: check types
        // ...

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(AnalyzeError::Yield { dependency });
        }

        Ok(())
    }
}
