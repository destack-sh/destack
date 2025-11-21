use dyst_dir::{LocalNodeId, LocalNodeIdAny, Session, Type};

use crate::{CompileError, CompileStage};

/// Error when validateing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ValidateError {
    /// Missing type for an expression.
    MissingType { node: LocalNodeIdAny },
    /// Type is not assignable to the expected type.
    TypeMismatch {
        node: LocalNodeIdAny,
        expected_ty: LocalNodeId<Type>,
        actual_ty: LocalNodeId<Type>,
    },
    /// Calling non-callable.
    NonCallable { node: LocalNodeIdAny },
    /// Indexing non-indexable.
    NonIndexable { node: LocalNodeIdAny },
    /// Non-exhaustive match/switch when exhaustiveness is required.
    NonExhaustiveMatch { node: LocalNodeIdAny },
    /// Incomplete pattern.
    IncompletePattern { node: LocalNodeIdAny },
    /// Missing return on code paths in functions that must return a value.
    MissingReturn { node: LocalNodeIdAny },
    /// Use of uninitialized variable in a read position.
    UninitializedVariable { node: LocalNodeIdAny },
    /// Illegal casts (unsafe or impossible with static rules).
    IllegalCast {
        node: LocalNodeIdAny,
        from_ty: LocalNodeId<Type>,
        to_ty: LocalNodeId<Type>,
    },
}

impl ValidateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingType { .. } => 2,
            Self::TypeMismatch { .. } => 3,
            Self::NonCallable { .. } => 4,
            Self::NonIndexable { .. } => 5,
            Self::NonExhaustiveMatch { .. } => 6,
            Self::IncompletePattern { .. } => 7,
            Self::MissingReturn { .. } => 8,
            Self::UninitializedVariable { .. } => 9,
            Self::IllegalCast { .. } => 10,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::MissingType { node, .. } => Some(*node),
            Self::TypeMismatch { node, .. } => Some(*node),
            Self::NonCallable { node, .. } => Some(*node),
            Self::NonIndexable { node, .. } => Some(*node),
            Self::NonExhaustiveMatch { node, .. } => Some(*node),
            Self::IncompletePattern { node, .. } => Some(*node),
            Self::MissingReturn { node, .. } => Some(*node),
            Self::UninitializedVariable { node, .. } => Some(*node),
            Self::IllegalCast { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::MissingType { .. } => "missing type".to_string(),
            Self::TypeMismatch { .. } => "type mismatch".to_string(),
            Self::NonCallable { .. } => "calling non-callable".to_string(),
            Self::NonIndexable { .. } => "indexing non-indexable".to_string(),
            Self::NonExhaustiveMatch { .. } => "non-exhaustive match".to_string(),
            Self::IncompletePattern { .. } => "incomplete pattern".to_string(),
            Self::MissingReturn { .. } => "missing return".to_string(),
            Self::UninitializedVariable { .. } => "uninitialized variable".to_string(),
            Self::IllegalCast { .. } => "illegal cast".to_string(),
        }
    }
}

impl std::fmt::Display for ValidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidateError")
            .field(
                "code",
                &format!(
                    "{}E{:03}",
                    CompileStage::Validate.letter(),
                    self.sub_code()
                ),
            )
            .finish()
    }
}

impl From<ValidateError> for CompileError {
    #[inline]
    fn from(error: ValidateError) -> Self {
        CompileError::Validate(error)
    }
}

pub type ValidateResult<T> = Result<T, ValidateError>;
