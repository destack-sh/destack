use crate::{Compiler, LinkResult, Task, TaskDebug, TaskOutput};

use destack_source::ModuleId;
use destack_workspace::Program;

/// Task to link something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum LinkTask {
    /// Link a module.
    Link { module: ModuleId },
}

impl LinkTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Link { .. } => 1,
        }
    }
}

impl TaskDebug for LinkTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Link { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Link { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<LinkTask> for Task {
    fn from(task: LinkTask) -> Self {
        Task::Link(task)
    }
}

/// Output of a link task.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkOutput {}

impl From<LinkOutput> for TaskOutput {
    fn from(output: LinkOutput) -> Self {
        TaskOutput::Link(output)
    }
}

impl Compiler {
    /// Process a link task.
    pub fn process_link(&self, task: LinkTask) -> LinkResult<LinkOutput> {
        todo!("process_link({task:?})")
    }
}
