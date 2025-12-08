use crate::{Compiler, OptimizeResult, Task, TaskDebug, TaskDependencyError, TaskOutput};

use destack_source::ModuleId;
use destack_workspace::Program;

/// Task to optimize something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum OptimizeTask {
    /// Optimize a module's MIR.
    OptimizeModule { module: ModuleId },
}

impl OptimizeTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::OptimizeModule { .. } => 1,
        }
    }
}

impl TaskDebug for OptimizeTask {
    fn name(&self) -> &'static str {
        match self {
            Self::OptimizeModule { .. } => "optimize module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::OptimizeModule { module } => {
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
    /// Process an optimize task.
    pub fn process_optimize(&self, task: OptimizeTask) -> OptimizeResult<OptimizeOutput> {
        match task {
            OptimizeTask::OptimizeModule { module } => {
                self.require_verify(module)?;
                self.optimize_module(module)?;
            }
        }
        Ok(OptimizeOutput {})
    }

    /// Optimize a module's MIR.
    fn optimize_module(&self, _module: ModuleId) -> OptimizeResult<()> {
        // NOTE #Incomplete: implement optimize
        // - dead code elimination
        // - constant folding
        // - inlining
        Ok(())
    }

    /// Ensure a module has been optimized.
    pub fn require_optimize(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.require_task(OptimizeTask::OptimizeModule { module })
    }
}
