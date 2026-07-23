use std::{error, fmt};

use destack_program::{BindingId, FunctionId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One runtime binding execution failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum BindingError {
    /// The binding has no VM implementation.
    Unavailable {
        /// The bound function.
        function: FunctionId,
        /// The unavailable runtime binding.
        binding: BindingId,
    },
}

impl fmt::Display for BindingError {
    /// Format one runtime binding execution failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { function, binding } => {
                write!(formatter, "{binding:?} for {function:?} is unavailable")
            }
        }
    }
}

impl error::Error for BindingError {}
