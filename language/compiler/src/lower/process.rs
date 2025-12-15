use crate::{Compiler, LowerResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

/// Task to lower a DIR into MIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Lower)]
pub enum LowerTask {
    /// Lower a module into MIR.
    #[task(code = 1, trace = "module={module}")]
    LowerModule { module: ModuleId },
}

impl Compiler {
    /// Process a lower task.
    pub fn process_lower(&self, task: LowerTask) -> LowerResult<()> {
        match task {
            LowerTask::LowerModule { module } => {
                self.require_elaborate_module(module)?;
                self.lower_module(module)?;
            }
        }
        Ok(())
    }

    /// Lower a module.
    fn lower_module(&self, _module: ModuleId) -> LowerResult<()> {
        // NOTE #Incomplete: implement lower
        Ok(())
    }

    /// Ensure a module has been lowered.
    pub fn require_lower_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LowerTask::LowerModule { module })
    }
}
