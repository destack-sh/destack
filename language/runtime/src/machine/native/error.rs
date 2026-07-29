use std::fmt;

use destack_core::StringId;
use destack_program as program;
use destack_program::native::{NativeExitError, NativeExitKind, NativeTrap, NativeTrapError};
use destack_program::{TypeId, Value};
use serde::{Deserialize, Serialize};

/// Native execution error.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    /// The requested entry is not present in the native program.
    EntryNotFound {
        /// The missing entry name.
        name: String,
    },
    /// One native call supplied the wrong number of source values.
    ArgumentCount {
        /// Number of values selected by the Program signature.
        expected: usize,
        /// Number of values supplied by the runtime call.
        actual: usize,
    },
    /// One native call references a type without physical Program metadata.
    TypeMissing {
        /// Missing Program type.
        ty: TypeId,
    },
    /// Native execution reported a trap.
    Trapped {
        /// The reported trap.
        trap: NativeTrap,
    },
    /// Native execution exited without a restorable machine state.
    StateUnavailable {
        /// The exit that requires machine state.
        kind: NativeExitKind,
        /// The safepoint that exited.
        safepoint: u32,
    },
    /// Native execution reported a language panic.
    Panicked {
        /// The optional panic payload.
        payload: Option<Value>,
    },
    /// A native exit code could not be decoded.
    InvalidExit(NativeExitError),
    /// A native trap code could not be decoded.
    InvalidTrap(NativeTrapError),
    /// Program metadata rejected one native call value.
    Program(Box<program::Error>),
    /// The program has no native code.
    NativeCodeMissing,
    /// A program string id could not be resolved.
    ProgramStringMissing {
        /// Missing program string id.
        string: StringId,
    },
    /// A resident native symbol could not be resolved.
    NativeSymbolMissing {
        /// The missing native symbol.
        symbol: String,
    },
}

impl Clone for Error {
    /// Copy diagnostic state while explicitly sharing panic value storage.
    fn clone(&self) -> Self {
        match self {
            Self::EntryNotFound { name } => Self::EntryNotFound { name: name.clone() },
            Self::ArgumentCount { expected, actual } => Self::ArgumentCount {
                expected: *expected,
                actual: *actual,
            },
            Self::TypeMissing { ty } => Self::TypeMissing { ty: *ty },
            Self::Trapped { trap } => Self::Trapped { trap: *trap },
            Self::StateUnavailable { kind, safepoint } => Self::StateUnavailable {
                kind: *kind,
                safepoint: *safepoint,
            },
            Self::Panicked { payload } => Self::Panicked {
                payload: payload.as_ref().map(Value::fork),
            },
            Self::InvalidExit(error) => Self::InvalidExit(*error),
            Self::InvalidTrap(error) => Self::InvalidTrap(*error),
            Self::Program(error) => Self::Program(error.clone()),
            Self::NativeCodeMissing => Self::NativeCodeMissing,
            Self::ProgramStringMissing { string } => Self::ProgramStringMissing { string: *string },
            Self::NativeSymbolMissing { symbol } => Self::NativeSymbolMissing {
                symbol: symbol.clone(),
            },
        }
    }
}

impl fmt::Display for Error {
    /// Format one native execution error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryNotFound { name } => write!(formatter, "native entry not found: {name}"),
            Self::ArgumentCount { expected, actual } => {
                write!(
                    formatter,
                    "native call expected {expected} values but received {actual}"
                )
            }
            Self::TypeMissing { ty } => write!(formatter, "native type not found: {ty:?}"),
            Self::Trapped { trap } => {
                write!(formatter, "native execution trapped: {trap:?}")
            }
            Self::StateUnavailable { kind, safepoint } => {
                write!(
                    formatter,
                    "native execution exited with {kind:?} at safepoint {safepoint} without machine state"
                )
            }
            Self::Panicked { payload } => {
                write!(formatter, "native execution panicked with {payload:?}")
            }
            Self::InvalidExit(error) => write!(formatter, "native exit error: {error}"),
            Self::InvalidTrap(error) => write!(formatter, "native trap error: {error}"),
            Self::Program(error) => write!(formatter, "native program error: {error}"),
            Self::NativeCodeMissing => write!(formatter, "program has no native code"),
            Self::ProgramStringMissing { string } => {
                write!(formatter, "program string not found: {string:?}")
            }
            Self::NativeSymbolMissing { symbol } => {
                write!(formatter, "native symbol not found: {symbol}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<program::Error> for Error {
    /// Preserve one native program failure.
    fn from(error: program::Error) -> Self {
        Self::Program(Box::new(error))
    }
}
