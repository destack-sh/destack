use crate::{Compiler, LowerResult, Task, TaskOutput, TaskDebug};

use dyst_dir::{ModuleId, Program};

/// Task to lower a DIR into MIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LowerTask {
    /// Lower a module.
    Lower { module: ModuleId },
}

impl LowerTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Lower { .. } => 1,
        }
    }
}

impl TaskDebug for LowerTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Lower { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Lower { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone();
                format!("module={uri}")
            }
        }
    }
}

impl From<LowerTask> for Task {
    fn from(task: LowerTask) -> Self {
        Task::Lower(task)
    }
}

/// Output of a lower task.
#[derive(Debug, Clone, PartialEq)]
pub struct LowerOutput {}

impl From<LowerOutput> for TaskOutput {
    fn from(output: LowerOutput) -> Self {
        TaskOutput::Lower(output)
    }
}

impl Compiler {
    /// Process a lower task.
    pub fn process_lower(&self, task: LowerTask) -> LowerResult<LowerOutput> {
        todo!("process_lower({task:?})")
    }
}
