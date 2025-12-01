use crate::{VerifyResult, Compiler, Task, TaskDebug, TaskOutput};

use destack_dir::{ModuleId, Program};

/// Task to verify something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum VerifyTask {
    /// Verify a module.
    Verify { module: ModuleId },
}

impl VerifyTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Verify { .. } => 1,
        }
    }
}

impl TaskDebug for VerifyTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Verify { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Verify { module } => {
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
        todo!("process_verify({task:?})")
    }
}
