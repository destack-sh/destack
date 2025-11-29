use crate::{Compiler, OptimizeResult, Task, TaskDebug, TaskOutput};

use destack_dir::{ModuleId, Program};

/// Task to optimize something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum OptimizeTask {
    /// Optimize a module.
    Optimize { module: ModuleId },
}

impl OptimizeTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Optimize { .. } => 1,
        }
    }
}

impl TaskDebug for OptimizeTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Optimize { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Optimize { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<OptimizeTask> for Task {
    fn from(task: OptimizeTask) -> Self {
        Task::Optimize(task)
    }
}

/// Output of an optimize task.
#[derive(Debug, Clone, PartialEq)]
pub struct OptimizeOutput {}

impl From<OptimizeOutput> for TaskOutput {
    fn from(output: OptimizeOutput) -> Self {
        TaskOutput::Optimize(output)
    }
}

impl Compiler {
    /// Process a optimize task.
    pub fn process_optimize(&self, task: OptimizeTask) -> OptimizeResult<OptimizeOutput> {
        todo!("process_optimize({task:?})")
    }
}
