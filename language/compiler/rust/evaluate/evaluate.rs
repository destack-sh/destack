use crate::Compiler;

use dyst_dir::{Annotation, Argument, Expression, NodeId, Type};

/// Task to statically evaluate something in-place.
#[derive(Debug, Clone)]
pub enum EvaluateTask {
    /// Evaluate an Expression fully (in-place).
    EvaluateExpression { expression: NodeId<Expression> },
    /// Evaluate a Type to its Type value (in-place).
    EvaluateType { ty: NodeId<Type> },
    /// Evaluate an Argument (in-place).
    EvaluateArgument { argument: NodeId<Argument> },
    /// Evaluate an Annotation fully (in-place).
    EvaluateAnnotation { annotation: NodeId<Annotation> },
}

impl<'a> Compiler<'a> {
    /// Evaluate a node.
    pub fn process_evaluate(&mut self, task: EvaluateTask) {
        let _result = match task {
            EvaluateTask::EvaluateExpression { expression } => self.evaluate_expression(expression),
            EvaluateTask::EvaluateType { ty } => self.evaluate_type(ty),
            EvaluateTask::EvaluateArgument { argument } => self.evaluate_argument(argument),
            EvaluateTask::EvaluateAnnotation { annotation } => self.evaluate_annotation(annotation),
        };
    }
}
