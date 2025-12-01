use crate::{Compiler, GenerateResult, Task, TaskDebug, TaskOutput};

use destack_dir::{ModuleId, Program};

/// task to generate something into an artifact.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum GenerateTask {
    /// Generate a module.
    Generate { module: ModuleId },
}

impl GenerateTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Generate { .. } => 1,
        }
    }
}

impl TaskDebug for GenerateTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Generate { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Generate { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<GenerateTask> for Task {
    fn from(task: GenerateTask) -> Self {
        Task::Generate(task)
    }
}

/// Output of a generate task.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateOutput {}

impl From<GenerateOutput> for TaskOutput {
    fn from(output: GenerateOutput) -> Self {
        TaskOutput::Generate(output)
    }
}

impl Compiler {
    /// Process a generate task.
    pub fn process_generate(&self, task: GenerateTask) -> GenerateResult<GenerateOutput> {
        todo!("process_generate({task:?})")
    }
}
