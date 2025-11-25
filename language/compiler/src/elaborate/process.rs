use crate::{CompileTask, Compiler, ElaborateResult};

use dyst_dir::{ModuleId, Program};

/// Task to elaborate something.
#[derive(Debug, Clone)]
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

    /// Get a message for the task.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::Elaborate { module } => {
                format!("elaborate module '{module:?}'")
            }
        }
    }
}

impl From<ElaborateTask> for CompileTask {
    fn from(task: ElaborateTask) -> Self {
        CompileTask::Elaborate(task)
    }
}

/// Output of an elaborate task.
#[derive(Debug, Clone)]
pub struct ElaborateOutput {}

impl<'a> Compiler<'a> {
    /// Process a elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<ElaborateOutput> {
        todo!("process_elaborate({task:?})")
    }
}
