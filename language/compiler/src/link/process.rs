use crate::{CompileOutput, CompileTask, Compiler, LinkResult};

use dyst_dir::{ModuleId, Program};

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

    /// Get a message for the task.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Link { module } => {
                format!("link module '{module:?}'")
            }
        }
    }
}

impl From<LinkTask> for CompileTask {
    fn from(task: LinkTask) -> Self {
        CompileTask::Link(task)
    }
}

/// Output of a link task.
#[derive(Debug, Clone)]
pub struct LinkOutput {}

impl From<LinkOutput> for CompileOutput {
    fn from(output: LinkOutput) -> Self {
        CompileOutput::Link(output)
    }
}

impl Compiler {
    /// Process a link task.
    pub fn process_link(&self, task: LinkTask) -> LinkResult<LinkOutput> {
        todo!("process_link({task:?})")
    }
}
