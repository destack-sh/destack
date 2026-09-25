use std::fmt;

use serde::{Deserialize, Serialize};
use tspp_native::abi;
use tspp_program as program;
use tspp_program::{TypeId, Value};

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
        trap: abi::Trap,
    },
    /// Native execution exited without a restorable machine state.
    StateUnavailable {
        /// The exit that requires machine state.
        kind: abi::ExitKind,
        /// The native frame map that exited.
        frame_map: u32,
    },
    /// Native execution reported a language panic.
    Panicked {
        /// Optional language panic payload.
        payload: Option<Value>,
    },
    /// The platform rejected the native trap handler.
    Signal {
        /// The rejected signal.
        signal: i32,
        /// The platform error code.
        code: Option<i32>,
    },
    /// Program metadata rejected one native call value.
    Program(Box<program::Error>),
    /// The program has no native code.
    NativeCodeMissing,
    /// The linked code requires a different native runtime ABI.
    AbiVersion {
        /// Runtime ABI version.
        expected: u32,
        /// Linked code ABI version.
        actual: u32,
    },
    /// One linked native range escapes its executable image.
    NativeImageRange,
    /// The executable mapping does not satisfy the linked image alignment.
    NativeImageMisaligned {
        /// Required executable base alignment.
        required: u32,
    },
    /// One linked native function has no callable entry bytes.
    NativeEntryEmpty {
        /// Program function with the empty entry.
        function: program::FunctionId,
    },
    /// The platform could not map, protect, or register native code.
    NativeLoad {
        /// Failed platform operation.
        operation: LoadOperation,
        /// Platform diagnostic.
        message: String,
    },
}

/// One platform native loading operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadOperation {
    /// Reserve writable memory.
    Map,
    /// Make generated code executable.
    Protect,
    /// Register platform unwind tables.
    Register,
}

impl Clone for Error {
    /// Copy native execution diagnostic state.
    fn clone(&self) -> Self {
        match self {
            Self::EntryNotFound { name } => Self::EntryNotFound { name: name.clone() },
            Self::ArgumentCount { expected, actual } => Self::ArgumentCount {
                expected: *expected,
                actual: *actual,
            },
            Self::TypeMissing { ty } => Self::TypeMissing { ty: *ty },
            Self::Trapped { trap } => Self::Trapped { trap: *trap },
            Self::StateUnavailable { kind, frame_map } => Self::StateUnavailable {
                kind: *kind,
                frame_map: *frame_map,
            },
            Self::Panicked { payload } => Self::Panicked {
                payload: payload.as_ref().map(Value::fork),
            },
            Self::Signal { signal, code } => Self::Signal {
                signal: *signal,
                code: *code,
            },
            Self::Program(error) => Self::Program(error.clone()),
            Self::NativeCodeMissing => Self::NativeCodeMissing,
            Self::AbiVersion { expected, actual } => Self::AbiVersion {
                expected: *expected,
                actual: *actual,
            },
            Self::NativeImageRange => Self::NativeImageRange,
            Self::NativeImageMisaligned { required } => Self::NativeImageMisaligned {
                required: *required,
            },
            Self::NativeEntryEmpty { function } => Self::NativeEntryEmpty {
                function: *function,
            },
            Self::NativeLoad { operation, message } => Self::NativeLoad {
                operation: *operation,
                message: message.clone(),
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
            Self::StateUnavailable { kind, frame_map } => {
                write!(
                    formatter,
                    "native execution exited with {kind:?} at frame map {frame_map} without machine state"
                )
            }
            Self::Panicked { payload } => {
                write!(formatter, "native execution panicked with {payload:?}")
            }
            Self::Signal { signal, code } => {
                write!(
                    formatter,
                    "native trap handler rejected for signal {signal} (code {code:?})"
                )
            }
            Self::Program(error) => write!(formatter, "native program error: {error}"),
            Self::NativeCodeMissing => write!(formatter, "program has no native code"),
            Self::AbiVersion { expected, actual } => {
                write!(
                    formatter,
                    "native code requires ABI {actual}, runtime provides ABI {expected}"
                )
            }
            Self::NativeImageRange => {
                formatter.write_str("native code range escapes its executable image")
            }
            Self::NativeImageMisaligned { required } => {
                write!(formatter, "native code requires {required}-byte alignment")
            }
            Self::NativeEntryEmpty { function } => {
                write!(formatter, "native function {function:?} has an empty entry")
            }
            Self::NativeLoad { operation, message } => {
                write!(formatter, "native {operation:?} failed: {message}")
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
