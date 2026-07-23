use std::{error, fmt};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One VM state or host compatibility failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MachineError {
    /// A continuation does not match the linked frame tables.
    InvalidContinuation,
    /// The Program pointer width differs from the host pointer width.
    IncompatiblePointerWidth {
        /// The Program pointer width in bytes.
        program: u8,
        /// The host pointer width in bytes.
        host: u8,
    },
}

impl fmt::Display for MachineError {
    /// Format one VM state or host compatibility failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContinuation => formatter.write_str("invalid continuation"),
            Self::IncompatiblePointerWidth { program, host } => write!(
                formatter,
                "program uses {program}-byte pointers on a {host}-byte host"
            ),
        }
    }
}

impl error::Error for MachineError {}
