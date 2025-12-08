use crate::{Compiler, Task, TaskDebug, TaskDependencyError, TaskOutput, VerifyResult};

use destack_source::ModuleId;
use destack_workspace::Program;

/// Task to verify something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum VerifyTask {
    /// Verify a module.
    VerifyModule { module: ModuleId },
}

impl VerifyTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::VerifyModule { .. } => 1,
        }
    }
}

impl TaskDebug for VerifyTask {
    fn name(&self) -> &'static str {
        match self {
            Self::VerifyModule { .. } => "verify module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::VerifyModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<VerifyTask> for Task {
    fn from(task: VerifyTask) -> Self {
        Task::Verify(task)
    }
}

/// Output of an verify task.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifyOutput {}

impl From<VerifyOutput> for TaskOutput {
    fn from(output: VerifyOutput) -> Self {
        TaskOutput::Verify(output)
    }
}

impl Compiler {
    /// Process a verify task.
    pub fn process_verify(&self, task: VerifyTask) -> VerifyResult<VerifyOutput> {
        match task {
            VerifyTask::VerifyModule { module } => {
                self.require_lower(module)?;
                self.verify_module(module)?;
            }
        }
        Ok(VerifyOutput {})
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
    pub fn require_verify(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.require_task(VerifyTask::VerifyModule { module })
    }
}
