use crate::Compiler;

use dyst_dir::{NodeId, Pattern, Type};

/// Task to validate something.
#[derive(Debug, Clone)]
pub enum ValidateTask {
    /// Typevalidate a Type.
    ValidateType { node: NodeId<Type> },
    /// Validate a Pattern.
    ValidatePattern { node: NodeId<Pattern> },
}

impl<'a> Compiler<'a> {
    /// Process a validate task.
    pub fn process_validate(&mut self, task: ValidateTask) {
        todo!("process_validate({task:?})")
    }
}
