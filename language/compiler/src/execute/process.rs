use crate::{Compiler, ExecuteResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

/// Task to execute comptime code.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Execute)]
pub enum ExecuteTask {
    /// Execute comptime code for a module.
    #[task(code = 1, trace = "module={module}")]
    ExecuteModule { module: ModuleId },
}

impl Compiler {
    /// Process an execute task.
    pub fn process_execute(&self, task: ExecuteTask) -> ExecuteResult<()> {
        match task {
            ExecuteTask::ExecuteModule { module } => {
                // (placeholder, not sure yet how to structure comptime execution)
                todo!("#Incomplete: execute comptime code for module {module}");
            }
        }
    }

    /// Ensure a module's comptime code has been executed.
    pub fn require_execute(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ExecuteTask::ExecuteModule { module })
    }
}
