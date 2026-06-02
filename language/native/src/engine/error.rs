use std::fmt;

use destack_engine::{EngineId, Value};

use crate::{NativeStatusError, NativeTrap, NativeTrapError, NativeValueError};

/// Native backend execution error.
#[derive(Debug)]
pub enum Error {
    /// The requested entry is not present in the native program.
    EntryNotFound {
        /// The missing entry name.
        name: String,
    },
    /// Native execution yielded without a materialized continuation.
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
    /// A native exit status code could not be decoded.
    InvalidStatus(NativeStatusError),
    /// A native trap code could not be decoded.
    InvalidTrap(NativeTrapError),
    /// A native ABI value could not be decoded.
    Value(NativeValueError),
    /// Native continuation state is not resumable by this engine.
    ContinuationUnavailable,
    /// A native image belongs to another engine.
    ImageEngineMismatch {
        /// The current engine id.
        engine_id: EngineId,
        /// The captured image engine id.
        image_engine_id: EngineId,
    },
    /// Native root metadata is not available.
    RootMapUnavailable,
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
            Self::InvalidStatus(error) => write!(formatter, "native status error: {error}"),
            Self::InvalidTrap(error) => write!(formatter, "native trap error: {error}"),
            Self::Value(error) => write!(formatter, "native value error: {error}"),
            Self::ContinuationUnavailable => {
                write!(formatter, "native continuation is not resumable")
            }
            Self::ImageEngineMismatch {
                engine_id,
                image_engine_id,
            } => write!(
                formatter,
                "native image belongs to engine {}, not {}",
                image_engine_id.get(),
                engine_id.get()
            ),
            Self::RootMapUnavailable => write!(formatter, "native root map is not available"),
        }
    }
}

impl std::error::Error for Error {}
