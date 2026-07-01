use std::fmt;

use destack_core::StringId;
use destack_program::native::{
    NativeContinuationError, NativeExitError, NativeMaterializationError, NativeTrap,
    NativeTrapError, NativeValueError,
};
use destack_program::{FrameStateId, Value};

/// Native execution error.
#[derive(Debug)]
pub enum Error {
    /// The requested entry is not present in the native program.
    EntryNotFound {
        /// The missing entry name.
        name: String,
    },
    /// Native execution yielded without a continuation.
    YieldedWithoutContinuation {
        /// The safepoint that yielded.
        safepoint: u32,
    },
    /// Native execution reported a trap.
    Trapped {
        /// The reported trap.
        trap: NativeTrap,
    },
    /// Native execution requested deoptimization without materialization.
    DeoptimizedWithoutMaterialization {
        /// The safepoint that requested deoptimization.
        safepoint: u32,
    },
    /// Native execution reported a language panic.
    Panicked {
        /// The panic payload.
        payload: Value,
    },
    /// A native exit code could not be decoded.
    InvalidExit(NativeExitError),
    /// A native trap code could not be decoded.
    InvalidTrap(NativeTrapError),
    /// A native ABI value could not be decoded.
    Value(NativeValueError),
    /// A native materialization could not be decoded.
    InvalidMaterialization(NativeMaterializationError),
    /// A native continuation could not be decoded.
    InvalidContinuation(NativeContinuationError),
    /// A native continuation frame does not match program frame metadata.
    InvalidContinuationFrame {
        /// The invalid frame state.
        frame_state: FrameStateId,
    },
    /// Native continuation has no frame to resume.
    EmptyContinuation,
    /// Native continuation has no resume entry for its frame state.
    ResumeEntryNotFound {
        /// The missing frame state.
        frame_state: FrameStateId,
    },
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

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryNotFound { name } => write!(formatter, "native entry not found: {name}"),
            Self::YieldedWithoutContinuation { safepoint } => {
                write!(
                    formatter,
                    "native execution yielded at safepoint {safepoint} without a continuation"
                )
            }
            Self::Trapped { trap } => {
                write!(formatter, "native execution trapped: {trap:?}")
            }
            Self::DeoptimizedWithoutMaterialization { safepoint } => {
                write!(
                    formatter,
                    "native execution deoptimized at safepoint {safepoint} without materialization"
                )
            }
            Self::Panicked { payload } => {
                write!(formatter, "native execution panicked with {payload:?}")
            }
            Self::InvalidExit(error) => write!(formatter, "native exit error: {error}"),
            Self::InvalidTrap(error) => write!(formatter, "native trap error: {error}"),
            Self::Value(error) => write!(formatter, "native value error: {error}"),
            Self::InvalidMaterialization(error) => {
                write!(formatter, "native materialization error: {error}")
            }
            Self::InvalidContinuation(error) => {
                write!(formatter, "native continuation error: {error}")
            }
            Self::InvalidContinuationFrame { frame_state } => {
                write!(
                    formatter,
                    "native continuation frame does not match frame state {frame_state:?}"
                )
            }
            Self::EmptyContinuation => write!(formatter, "native continuation is empty"),
            Self::ResumeEntryNotFound { frame_state } => {
                write!(
                    formatter,
                    "native resume entry not found for {frame_state:?}"
                )
            }
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
