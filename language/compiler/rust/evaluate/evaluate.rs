use crate::Compiler;

use dyst_dir::{Annotation, Argument, Expression, NodeId, NodeIdAny, Type};

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

/// Error when evaluating something statically.
#[derive(Debug, Clone)]
pub enum EvaluateError {
    NotReady {
        node_id: NodeIdAny,
        depends_on: Option<NodeIdAny>,
    },
}

impl std::fmt::Display for EvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub type EvaluateResult<T> = Result<T, EvaluateError>;

impl<'a> Compiler<'a> {
    /// Evaluate a node.
    pub fn process_evaluate(&mut self, task: EvaluateTask) {
        let result = match task {
            EvaluateTask::EvaluateExpression { expression } => self.evaluate_expression(expression),
            EvaluateTask::EvaluateType { ty } => self.evaluate_type(ty),
            EvaluateTask::EvaluateArgument { argument } => self.evaluate_argument(argument),
            EvaluateTask::EvaluateAnnotation { annotation } => self.evaluate_annotation(annotation),
        };
    }
}
