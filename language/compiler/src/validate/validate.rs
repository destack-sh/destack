use crate::{Compiler, CompilerTask, ValidateResult};

use dyst_dir::{ModuleId, NodeId, Pattern, Type};

/// Task to validate something.
#[derive(Debug, Clone)]
pub enum ValidateTask {
    /// Validate a Type.
    ValidateType {
        module_id: ModuleId,
        ty: NodeId<Type>,
    },
    /// Validate a Pattern.
    ValidatePattern {
        module_id: ModuleId,
        pattern: NodeId<Pattern>,
    },
}

impl From<ValidateTask> for CompilerTask {
    fn from(task: ValidateTask) -> Self {
        CompilerTask::Validate(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a validate task.
    pub fn process_validate(&mut self, task: ValidateTask) -> ValidateResult<()> {
        todo!("process_validate({task:?})")
    }
}
