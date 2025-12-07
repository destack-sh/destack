use crate::{Compiler, ElaborateResult, Task, TaskDebug, TaskDependencyError, TaskOutput};

use destack_source::ModuleId;
use destack_workspace::Program;

/// Task to elaborate something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ElaborateTask {
    /// Elaborate a module.
    ElaborateModule { module: ModuleId },
}

impl ElaborateTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ElaborateModule { .. } => 1,
        }
    }
}

impl TaskDebug for ElaborateTask {
    fn name(&self) -> &'static str {
        match self {
            Self::ElaborateModule { .. } => "elaborate module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::ElaborateModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<ElaborateTask> for Task {
    fn from(task: ElaborateTask) -> Self {
        Task::Elaborate(task)
    }
}

/// Output of an elaborate task.
#[derive(Debug, Clone, PartialEq)]
pub struct ElaborateOutput {}

impl From<ElaborateOutput> for TaskOutput {
    fn from(output: ElaborateOutput) -> Self {
        TaskOutput::Elaborate(output)
    }
}

impl Compiler {
    /// Process an elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<ElaborateOutput> {
        match task {
            ElaborateTask::ElaborateModule { module } => {
                self.ensure_analyzed(module)?;
                self.elaborate_module(module)?;
            }
        }
        Ok(ElaborateOutput {})
    }

    /// Elaborate a module.
    fn elaborate_module(&self, _module: ModuleId) -> ElaborateResult<()> {
        // NOTE #Incomplete: implement elaboration
        // - monomorphization
        // - comptime evaluation
        // - desugaring
        // - ..?
        Ok(())
    }

    /// Ensure a module has been elaborated.
    pub fn ensure_elaborated(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.require_task(ElaborateTask::ElaborateModule { module })
    }
}
