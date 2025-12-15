use crate::{Compiler, TaskDependencyError, VerifyResult};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

/// Task to verify something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Verify)]
pub enum VerifyTask {
    /// Verify a module.
    #[task(code = 1, trace = "module={module}")]
    VerifyModule { module: ModuleId },
}

impl Compiler {
    /// Process a verify task.
    pub fn process_verify(&self, task: VerifyTask) -> VerifyResult<()> {
        match task {
            VerifyTask::VerifyModule { module } => {
                self.require_lower_module(module)?;
                self.verify_module(module)?;
            }
        }
        Ok(())
    }

    /// Verify a module's MIR.
    fn verify_module(&self, _module: ModuleId) -> VerifyResult<()> {
        // NOTE #Incomplete: implement verify
        // - ownership/borrow checking
        // - type checking at MIR level?
        // - ..?
        Ok(())
    }

    /// Ensure a module has been verified.
    pub fn require_verify_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(VerifyTask::VerifyModule { module })
    }
}
