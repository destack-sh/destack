use crate::Compiler;

use dyst_dir::{Annotation, Argument, Expression, ModuleId, NodeId, Type};

/// Task to statically evaluate something in-place.
#[derive(Debug, Clone)]
pub enum EvaluateTask {
    /// Evaluate an Expression fully (in-place).
    EvaluateExpression {
        module_id: ModuleId,
        expression: NodeId<Expression>,
    },
    /// Evaluate a Type to its Type value (in-place).
    EvaluateType {
        module_id: ModuleId,
        ty: NodeId<Type>,
    },
    /// Evaluate an Argument (in-place).
    EvaluateArgument {
        module_id: ModuleId,
        argument: NodeId<Argument>,
    },
    /// Evaluate an Annotation fully (in-place).
    EvaluateAnnotation {
        module_id: ModuleId,
        annotation: NodeId<Annotation>,
    },
}

impl<'a> Compiler<'a> {
    /// Evaluate a node.
    pub fn process_evaluate(&mut self, task: EvaluateTask) {
        todo!("process_evaluate({task:?})")
    }
}
