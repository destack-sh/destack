use std::fmt;

use destack_heap::{DropId, HeapError, TraceTableError};

use crate::{
    FrameLayoutId, FrameSlotId, FrameStateId, FunctionId, GlobalId, LayoutId, Signature,
    SignatureId, TypeId, ValueMismatch,
};

/// Program operation result.
pub type Result<T> = std::result::Result<T, Error>;

/// Program operation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A runtime value does not match its program type.
    ValueMismatch {
        /// The mismatched value tags.
        mismatch: ValueMismatch,
    },
    /// An integer value does not match its program width.
    IntegerWidthMismatch {
        /// The required integer width.
        expected: u8,
        /// The supplied integer width.
        actual: u16,
    },
    /// A character word is not a Unicode scalar value.
    InvalidCharacter {
        /// The invalid Unicode code point.
        code_point: u32,
    },
    /// A runtime value uses an unsupported multiword representation.
    UnsupportedValue {
        /// The unsupported program type.
        ty: TypeId,
    },
    /// A bytecode value has the wrong number of words.
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
    /// A static global has no storage in its selected static space.
    MissingGlobalStorage {
        /// The global without storage.
        global: GlobalId,
    },
    /// A continuation contains no captured frames.
    EmptyContinuation,
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
    /// A continuation frame names bytes outside its byte storage.
    InvalidFrameRange {
        /// The frame state whose byte range is invalid.
        frame_state: FrameStateId,
    },
    /// A continuation frame byte width differs from its frame layout.
    FrameByteLengthMismatch {
        /// The mismatched frame state.
        frame_state: FrameStateId,
        /// The frame layout byte width.
        expected: usize,
        /// The captured frame byte width.
        actual: usize,
    },
    /// A frame slot id does not name a slot in its frame layout.
    UndefinedFrameSlot {
        /// The containing frame layout.
        frame_layout: FrameLayoutId,
        /// The missing frame slot.
        slot: FrameSlotId,
    },
    /// A frame slot names bytes outside its captured frame.
    FrameSlotOutOfBounds {
        /// The containing frame state.
        frame_state: FrameStateId,
        /// The out-of-bounds frame slot.
        slot: FrameSlotId,
    },
    /// A frame state does not name a continuation site.
    UndefinedContinuationSite {
        /// The frame state without a continuation site.
        frame_state: FrameStateId,
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
}

impl Error {
    /// Return one value mismatch error.
    pub const fn value_mismatch(mismatch: ValueMismatch) -> Self {
        Self::ValueMismatch { mismatch }
    }

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

impl fmt::Display for Error {
    /// Format one Program operation failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueMismatch { mismatch } => mismatch.fmt(formatter),
            Self::IntegerWidthMismatch { expected, actual } => write!(
                formatter,
                "expected a {expected}-bit integer, found a {actual}-bit integer"
            ),
            Self::InvalidCharacter { code_point } => {
                write!(formatter, "invalid Unicode code point {code_point:#x}")
            }
            Self::UnsupportedValue { ty } => {
                write!(formatter, "type {ty:?} is not a scalar program value")
            }
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
            Self::MissingGlobalStorage { global } => {
                write!(formatter, "global {global:?} has no static storage")
            }
            Self::EmptyContinuation => formatter.write_str("continuation contains no frames"),
            Self::UndefinedFrameState { frame_state } => {
                write!(formatter, "undefined frame state {frame_state:?}")
            }
            Self::UndefinedFrameLayout { frame_layout } => {
                write!(formatter, "undefined frame layout {frame_layout:?}")
            }
            Self::InvalidFrameRange { frame_state } => {
                write!(
                    formatter,
                    "frame state {frame_state:?} has an invalid byte range"
                )
            }
            Self::FrameByteLengthMismatch {
                frame_state,
                expected,
                actual,
            } => write!(
                formatter,
                "frame state {frame_state:?} requires {expected} bytes, found {actual}"
            ),
            Self::UndefinedFrameSlot { frame_layout, slot } => write!(
                formatter,
                "undefined frame slot {slot:?} in frame layout {frame_layout:?}"
            ),
            Self::FrameSlotOutOfBounds { frame_state, slot } => write!(
                formatter,
                "frame slot {slot:?} exceeds frame state {frame_state:?}"
            ),
            Self::UndefinedContinuationSite { frame_state } => {
                write!(
                    formatter,
                    "undefined continuation site for frame state {frame_state:?}"
                )
            }
            Self::Trace { error } => error.fmt(formatter),
            Self::Heap { error } => error.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {}
