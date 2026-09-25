use std::{error, fmt};

use serde::{Deserialize, Serialize};
use tspp_heap::{DropId, HeapError, TraceTableError};
use tspp_memory::MemoryError;

use crate::{
    FiberId, FrameLayoutId, FrameStateId, FunctionId, GlobalId, LayoutId, Signature, SignatureId,
    TypeId,
};

/// Result of one Program operation.
pub type Result<T> = std::result::Result<T, Error>;

/// One Program operation error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    /// A value carries the wrong concrete Program type.
    ValueTypeMismatch {
        /// The required program type.
        expected: TypeId,
        /// The supplied program type.
        actual: TypeId,
    },
    /// A value carries the wrong number of execution words.
    ValueWordCountMismatch {
        /// The runtime value type.
        ty: TypeId,
        /// The required word count.
        expected: usize,
        /// The supplied word count.
        actual: usize,
    },
    /// Function call signature does not match the target function.
    FunctionSignatureMismatch {
        /// The target function.
        function: FunctionId,
        /// Expected call signature.
        expected: Signature,
        /// Actual function signature.
        actual: Signature,
    },
    /// A function id does not name a Program function.
    UndefinedFunction {
        /// The missing function id.
        function: FunctionId,
    },
    /// A signature id does not name a Program signature.
    UndefinedSignature {
        /// The missing signature id.
        signature: SignatureId,
    },
    /// A type id does not name a Program type.
    UndefinedType {
        /// The missing type id.
        ty: TypeId,
    },
    /// A layout id does not name a Program layout.
    UndefinedLayout {
        /// The missing layout id.
        layout: LayoutId,
    },
    /// A drop id does not name a Program destructor.
    UndefinedDrop {
        /// The missing drop id.
        drop: DropId,
    },
    /// A byte range does not match its runtime type layout.
    ByteLengthMismatch {
        /// The runtime value type.
        ty: TypeId,
        /// The layout byte length.
        expected: usize,
        /// The supplied byte length.
        actual: usize,
    },
    /// An allocation site targets the constant space.
    ConstantAllocationSite,
    /// A static global has no storage in its selected static space.
    MissingGlobalStorage {
        /// The global without storage.
        global: GlobalId,
    },
    /// A frame state id does not name a Program frame state.
    UndefinedFrameState {
        /// The missing frame state id.
        frame_state: FrameStateId,
    },
    /// A frame layout id does not name a Program frame layout.
    UndefinedFrameLayout {
        /// The missing frame layout id.
        frame_layout: FrameLayoutId,
    },
    /// A retained call chain contains no frames.
    EmptyCallChain,
    /// A fiber handle does not name one live fiber.
    UndefinedFiber {
        /// The undefined fiber identity.
        fiber_id: FiberId,
    },
    /// A fiber operation is invalid for its current execution state.
    InvalidFiberState {
        /// The fiber identity in the invalid state.
        fiber_id: FiberId,
    },
    /// A retained frame byte width differs from its frame layout.
    FrameByteLengthMismatch {
        /// The mismatched frame state.
        frame_state: FrameStateId,
        /// The frame layout byte width.
        expected: usize,
        /// The captured frame byte width.
        actual: usize,
    },
    /// A canonical frame slot names bytes outside its retained frame.
    FrameSlotOutOfBounds {
        /// The containing frame state.
        frame_state: FrameStateId,
        /// The containing frame layout.
        frame_layout: FrameLayoutId,
        /// The out-of-bounds byte offset.
        offset: u32,
    },
    /// A frame address points outside every moved frame segment.
    StrayFrameAddress {
        /// The stray frame address.
        address: u64,
    },
    /// A compact trace table operation failed.
    Trace {
        /// The trace table failure.
        error: TraceTableError,
    },
    /// A heap operation failed.
    Heap {
        /// The heap failure.
        error: Box<HeapError>,
    },
    /// A memory operation failed.
    Memory {
        /// The memory failure.
        error: MemoryError,
    },
}

impl Error {
    /// Return one function signature mismatch error.
    pub fn function_signature_mismatch(
        function: FunctionId,
        expected: Signature,
        actual: Signature,
    ) -> Self {
        Self::FunctionSignatureMismatch {
            function,
            expected,
            actual,
        }
    }

    /// Return one undefined function error.
    pub fn undefined_function(function: FunctionId) -> Self {
        Self::UndefinedFunction { function }
    }

    /// Return one undefined signature error.
    pub fn undefined_signature(signature: SignatureId) -> Self {
        Self::UndefinedSignature { signature }
    }

    /// Return one undefined type error.
    pub fn undefined_type(ty: TypeId) -> Self {
        Self::UndefinedType { ty }
    }

    /// Return one undefined layout error.
    pub fn undefined_layout(layout: LayoutId) -> Self {
        Self::UndefinedLayout { layout }
    }

    /// Return one undefined destructor error.
    pub fn undefined_drop(drop: DropId) -> Self {
        Self::UndefinedDrop { drop }
    }
}

impl From<TraceTableError> for Error {
    /// Preserve one trace table failure.
    fn from(error: TraceTableError) -> Self {
        Self::Trace { error }
    }
}

impl From<HeapError> for Error {
    /// Preserve one heap failure.
    fn from(error: HeapError) -> Self {
        Self::Heap {
            error: Box::new(error),
        }
    }
}

impl From<MemoryError> for Error {
    /// Preserve one memory operation failure.
    fn from(error: MemoryError) -> Self {
        Self::Memory { error }
    }
}

impl fmt::Display for Error {
    /// Format one Program operation failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueTypeMismatch { expected, actual } => write!(
                formatter,
                "expected value type {}, found {}",
                expected.0, actual.0
            ),
            Self::ValueWordCountMismatch {
                ty,
                expected,
                actual,
            } => write!(
                formatter,
                "type {ty:?} requires {expected} value words, found {actual}"
            ),
            Self::FunctionSignatureMismatch { function, .. } => {
                write!(formatter, "function {function:?} has a different signature")
            }
            Self::UndefinedFunction { function } => {
                write!(formatter, "undefined function {function:?}")
            }
            Self::UndefinedSignature { signature } => {
                write!(formatter, "undefined signature {signature:?}")
            }
            Self::UndefinedType { ty } => write!(formatter, "undefined type {ty:?}"),
            Self::UndefinedLayout { layout } => {
                write!(formatter, "undefined layout {layout:?}")
            }
            Self::UndefinedDrop { drop } => write!(formatter, "undefined destructor {drop:?}"),
            Self::ByteLengthMismatch {
                ty,
                expected,
                actual,
            } => write!(
                formatter,
                "type {ty:?} requires {expected} bytes, found {actual}"
            ),
            Self::ConstantAllocationSite => {
                write!(formatter, "an allocation site targets the constant space")
            }
            Self::MissingGlobalStorage { global } => {
                write!(formatter, "global {global:?} has no static storage")
            }
            Self::UndefinedFrameState { frame_state } => {
                write!(formatter, "undefined frame state {frame_state:?}")
            }
            Self::UndefinedFrameLayout { frame_layout } => {
                write!(formatter, "undefined frame layout {frame_layout:?}")
            }
            Self::EmptyCallChain => formatter.write_str("retained call chain contains no frames"),
            Self::UndefinedFiber { fiber_id } => {
                write!(formatter, "undefined fiber {fiber_id:?}")
            }
            Self::InvalidFiberState { fiber_id } => {
                write!(formatter, "invalid state for fiber {fiber_id:?}")
            }
            Self::FrameByteLengthMismatch {
                frame_state,
                expected,
                actual,
            } => write!(
                formatter,
                "frame state {frame_state:?} requires {expected} bytes, found {actual}"
            ),
            Self::FrameSlotOutOfBounds {
                frame_state,
                frame_layout,
                offset,
            } => write!(
                formatter,
                "frame slot at byte {offset} exceeds layout {frame_layout:?} for state {frame_state:?}"
            ),
            Self::StrayFrameAddress { address } => {
                write!(
                    formatter,
                    "frame address {address:#x} lies outside every moved frame"
                )
            }
            Self::Trace { error } => error.fmt(formatter),
            Self::Heap { error } => error.fmt(formatter),
            Self::Memory { error } => error.fmt(formatter),
        }
    }
}

impl error::Error for Error {
    /// Return the underlying subsystem failure when present.
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Trace { error } => Some(error),
            Self::Heap { error } => Some(error.as_ref()),
            Self::Memory { error } => Some(error),
            _ => None,
        }
    }
}
