use crate::{Compiler, ElaborateResult, Task, TaskDebug, TaskOutput};

use dyst_dir::{ModuleId, Program};

/// Task to elaborate something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ElaborateTask {
    /// Elaborate a module.
    Elaborate { module: ModuleId },
}

impl ElaborateTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Elaborate { .. } => 1,
        }
    }
}

impl TaskDebug for ElaborateTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Elaborate { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Elaborate { module } => {
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
    /// Process a elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<ElaborateOutput> {
        todo!("process_elaborate({task:?})")
    }
}
