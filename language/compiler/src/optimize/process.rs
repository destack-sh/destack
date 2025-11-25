use crate::{CompileOutput, CompileTask, Compiler, OptimizeResult};

use dyst_dir::{ModuleId, Program};

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

    /// Get a message for the task.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::Optimize { module } => {
                format!("optimize module '{module:?}'")
            }
        }
    }
}

impl From<OptimizeTask> for CompileTask {
    fn from(task: OptimizeTask) -> Self {
        CompileTask::Optimize(task)
    }
}

/// Output of an optimize task.
#[derive(Debug, Clone)]
pub struct OptimizeOutput {}

impl From<OptimizeOutput> for CompileOutput {
    fn from(output: OptimizeOutput) -> Self {
        CompileOutput::Optimize(output)
    }
}

impl<'a> Compiler<'a> {
    /// Process a optimize task.
    pub fn process_optimize(&self, task: OptimizeTask) -> OptimizeResult<OptimizeOutput> {
        todo!("process_optimize({task:?})")
    }
}
