use crate::{Compiler, LowerResult, Task, TaskDebug, TaskDependencyError, TaskOutput};

use destack_source::ModuleId;
use destack_workspace::Program;

/// Task to lower a DIR into MIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LowerTask {
    /// Lower a module into MIR.
    LowerModule { module: ModuleId },
}

impl LowerTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::LowerModule { .. } => 1,
        }
    }
}

impl TaskDebug for LowerTask {
    fn name(&self) -> &'static str {
        match self {
            Self::LowerModule { .. } => "lower module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::LowerModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
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
        match task {
            LowerTask::LowerModule { module } => {
                self.require_elaborate_module(module)?;
                self.lower_module(module)?;
            }
        }
        Ok(LowerOutput {})
    }

    /// Lower a module.
    fn lower_module(&self, _module: ModuleId) -> LowerResult<()> {
        // NOTE #Incomplete: implement lower
        Ok(())
    }

    /// Ensure a module has been lowered.
    pub fn require_lower_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal(LowerTask::LowerModule { module })
    }
}
