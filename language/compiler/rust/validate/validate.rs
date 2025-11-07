use crate::Compiler;

use dyst_dir::{NodeId, NodeIdAny, Pattern, Type};

/// Request to validate something.
#[derive(Debug, Clone)]
pub enum ValidateTask {
    /// Typevalidate a Type.
    ValidateType { node: NodeId<Type> },
    /// Validate a Pattern.
    ValidatePattern { node: NodeId<Pattern> },
}

/// Error when validating something.
#[derive(Debug, Clone)]
pub enum ValidateError {
    NotReady {
        node_id: NodeIdAny,
        depends_on: Option<NodeIdAny>,
    },
}

pub type ValidateResult<T> = Result<T, ValidateError>;

impl<'a> Compiler<'a> {
    /// Process a validate request.
    pub fn process_validate(&mut self, request: ValidateTask) {
        todo!("process_validate({request:?})")
    }
}
