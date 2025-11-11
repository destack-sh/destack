use crate::Compiler;

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

impl<'a> Compiler<'a> {
    /// Process a validate task.
    pub fn process_validate(&mut self, task: ValidateTask) {
        todo!("process_validate({task:?})")
    }
}
