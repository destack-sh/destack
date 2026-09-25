use std::{error, fmt};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One VM state or host compatibility failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MachineError {
    /// The Program carries no executable bytecode.
    BytecodeUnavailable,
    /// A new execution was requested while stopped execution remains active.
    ExecutionActive,
    /// Continue was requested without stopped execution.
    ExecutionNotStopped,
    /// A machine image does not match the linked frame tables.
    InvalidImage,
    /// A detach boundary split could not relocate its suffix frames.
    InvalidSplit,
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
            Self::BytecodeUnavailable => formatter.write_str("program carries no bytecode"),
            Self::ExecutionActive => formatter.write_str("execution is already active"),
            Self::ExecutionNotStopped => formatter.write_str("execution is not stopped"),
            Self::InvalidImage => formatter.write_str("invalid machine image"),
            Self::InvalidSplit => formatter.write_str("invalid detach boundary split"),
            Self::IncompatiblePointerWidth { program, host } => write!(
                formatter,
                "program uses {program}-byte pointers on a {host}-byte host"
            ),
        }
    }
}

impl error::Error for MachineError {}
