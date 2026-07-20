use std::fmt;

use destack_heap::{HeapError, TraceTableError};

use crate::{FunctionId, GlobalId, LayoutId, Signature, SignatureId, TypeId};

/// Program operation result.
pub type Result<T> = std::result::Result<T, Error>;

/// Program operation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A runtime value does not match its program type.
    TypeMismatch { expected: String, actual: String },
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
    /// Return one type mismatch error.
    pub fn type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::TypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
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
            Self::TypeMismatch { expected, actual } => {
                write!(formatter, "expected {expected}, found {actual}")
            }
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
            Self::Trace { error } => error.fmt(formatter),
            Self::Heap { error } => error.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {}
