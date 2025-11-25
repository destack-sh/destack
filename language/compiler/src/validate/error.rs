use dyst_dir::{
    FunctionAbstraction, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Program, Visibility,
};

use crate::{CompileError, CompilePhase, CompileTaskWait};

/// Error when validateing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ValidateError {
    /// Wait for other tasks.
    Wait { wait: CompileTaskWait },
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

impl ValidateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Wait { .. } => 0,
            Self::MissingType { .. } => 1,
            Self::TypeMismatch { .. } => 2,
            Self::InaccessibleSymbol { .. } => 3,
            Self::InconsistentFunctionOverride { .. } => 4,
            Self::NonCallable { .. } => 5,
            Self::NonIndexable { .. } => 6,
            Self::NonExhaustiveMatch { .. } => 7,
            Self::IncompletePattern { .. } => 8,
            Self::MissingReturn { .. } => 9,
            Self::UninitializedVariable { .. } => 10,
            Self::IllegalCast { .. } => 11,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Wait { wait, .. } => wait.nodes.first().copied(),
            Self::MissingType { node, .. } => Some(*node),
            Self::TypeMismatch { node, .. } => Some(*node),
            Self::InaccessibleSymbol { node, .. } => Some(*node),
            Self::InconsistentFunctionOverride { node, .. } => Some(*node),
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
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::Wait { wait, .. } => {
                format!("wait for {} tasks", wait.tasks.len())
            }
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
                &format!("E{}{:03}", CompilePhase::Validate.letter(), self.sub_code()),
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
