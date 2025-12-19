use crate::{Compiler, ExecuteResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

/// Task to execute comptime code.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Execute)]
pub enum ExecuteTask {
    /// Execute comptime code for a module.
    #[task(code = 1, trace = "module={module}")]
    ExecuteModule { module: ModuleId }, // nocheckin: how to structure comptime execution? synthetic function?
}

impl Compiler {
    /// Process an execute task.
    pub fn process_execute(&self, task: ExecuteTask) -> ExecuteResult<()> {
        match task {
            ExecuteTask::ExecuteModule { module } => {
                self.require_verify_module(module)?;
                self.execute_module(module)?;
            }
        }
        Ok(())
    }

    /// Execute comptime code for a module.
    fn execute_module(&self, _module: ModuleId) -> ExecuteResult<()> {
        // NOTE #Incomplete: implement comptime execution
        // - evaluate comptime expressions
        // - substitute results back into MIR
        Ok(())
    }

    /// Ensure a module's comptime code has been executed.
    pub fn require_execute(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ExecuteTask::ExecuteModule { module })
    }
}
