use dyst_dir::{
    FunctionAbstraction, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Program, Visibility,
};

use crate::{Phase, TaskDependency, TaskError};

/// Error when validateing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ValidateError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    YieldFailed { dependency: TaskDependency },
    /// Missing type for an expression.
    MissingType { node: GlobalNodeIdAny },
    /// Type is not assignable to the expected type.
    TypeMismatch {
        node: GlobalNodeIdAny,
        expected_ty: GlobalTypeId,
        actual_ty: GlobalTypeId,
    },
    /// Inaccessible symbol (private/internal/module boundaries).
    InaccessibleSymbol {
        node: GlobalNodeIdAny,
        visibility: Visibility,
        symbol: GlobalSymbolId,
    },
    /// Inconsistent function override.
    InconsistentFunctionOverride {
        node: GlobalNodeIdAny,
        abstraction: FunctionAbstraction,
    },
    /// Calling non-callable.
    NonCallable { node: GlobalNodeIdAny },
    /// Indexing non-indexable.
    NonIndexable { node: GlobalNodeIdAny },
    /// Non-exhaustive match/switch when exhaustiveness is required.
    NonExhaustiveMatch { node: GlobalNodeIdAny },
    /// Incomplete pattern.
    IncompletePattern { node: GlobalNodeIdAny },
    /// Conflicting pattern arms.
    ConflictingPattern {
        node: GlobalNodeIdAny,
        other_node: Option<GlobalNodeIdAny>,
    },
    /// Missing return on code paths in functions that must return a value.
    MissingReturn { node: GlobalNodeIdAny },
    /// Use of uninitialized variable in a read position.
    UninitializedVariable { node: GlobalNodeIdAny },
    /// Illegal casts (unsafe or impossible with static rules).
    IllegalCast {
        node: GlobalNodeIdAny,
        from_ty: GlobalTypeId,
        to_ty: GlobalTypeId,
    },
}

impl TryFrom<ValidateError> for TaskDependency {
    type Error = ValidateError;

    fn try_from(error: ValidateError) -> Result<Self, Self::Error> {
        match error {
            ValidateError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl ValidateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::YieldFailed { .. } => 1,
            Self::MissingType { .. } => 2,
            Self::TypeMismatch { .. } => 3,
            Self::InaccessibleSymbol { .. } => 4,
            Self::InconsistentFunctionOverride { .. } => 5,
            Self::NonCallable { .. } => 6,
            Self::NonIndexable { .. } => 7,
            Self::NonExhaustiveMatch { .. } => 8,
            Self::IncompletePattern { .. } => 9,
            Self::ConflictingPattern { .. } => 10,
            Self::MissingReturn { .. } => 11,
            Self::UninitializedVariable { .. } => 12,
            Self::IllegalCast { .. } => 13,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::YieldFailed { dependency } => dependency.node(),
            Self::MissingType { node, .. } => *node,
            Self::TypeMismatch { node, .. } => *node,
            Self::InaccessibleSymbol { node, .. } => *node,
            Self::InconsistentFunctionOverride { node, .. } => *node,
            Self::NonCallable { node, .. } => *node,
            Self::NonIndexable { node, .. } => *node,
            Self::NonExhaustiveMatch { node, .. } => *node,
            Self::IncompletePattern { node, .. } => *node,
            Self::ConflictingPattern { node, .. } => *node,
            Self::MissingReturn { node, .. } => *node,
            Self::UninitializedVariable { node, .. } => *node,
            Self::IllegalCast { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::YieldFailed { .. } => "unsatisfied dependency".to_string(),
            Self::MissingType { .. } => "missing type".to_string(),
            Self::TypeMismatch { .. } => "type mismatch".to_string(),
            Self::InaccessibleSymbol { .. } => "inaccessible symbol".to_string(),
            Self::InconsistentFunctionOverride { .. } => {
                "inconsistent function override".to_string()
            }
            Self::NonCallable { .. } => "calling non-callable".to_string(),
            Self::NonIndexable { .. } => "indexing non-indexable".to_string(),
            Self::NonExhaustiveMatch { .. } => "non-exhaustive match".to_string(),
            Self::IncompletePattern { .. } => "incomplete pattern".to_string(),
            Self::ConflictingPattern { .. } => "conflicting pattern".to_string(),
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
                &format!("E{}{:03}", Phase::Validate.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ValidateError> for TaskError {
    #[inline]
    fn from(error: ValidateError) -> Self {
        TaskError::Validate(error)
    }
}

pub type ValidateResult<T> = Result<T, ValidateError>;
