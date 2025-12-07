use destack_dir::{FunctionAbstraction, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Visibility};
use destack_workspace::Program;

use crate::{TaskDependency, TaskDependencyError, TaskError, TaskPhase};

/// Error when analyzeing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum AnalyzeError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Unsupported node.
    UnsupportedConstruct { node: GlobalNodeIdAny },
    /// Missing type for an expression.
    MissingType { node: GlobalNodeIdAny },
    /// Type is not assignable to the expected type.
    UnassignableType {
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
    /// No overload found for operator/method with given types.
    NoOverload {
        node: GlobalNodeIdAny,
        receiver_ty: GlobalTypeId,
    },
    /// Ambiguous overload: multiple candidates match equally well.
    AmbiguousOverload {
        node: GlobalNodeIdAny,
        candidates: Vec<GlobalSymbolId>,
    },
    /// Operator not supported for type.
    UnsupportedOperator {
        node: GlobalNodeIdAny,
        ty: GlobalTypeId,
    },
}

impl From<TaskDependencyError> for AnalyzeError {
    fn from(e: TaskDependencyError) -> Self {
        match e {
            TaskDependencyError::NotReady { dependency } => Self::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                Self::UnsatisfiedDependency { dependency }
            }
        }
    }
}

impl TryFrom<AnalyzeError> for TaskDependency {
    type Error = AnalyzeError;

    fn try_from(error: AnalyzeError) -> Result<Self, Self::Error> {
        match error {
            AnalyzeError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl AnalyzeError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsatisfiedDependency { .. } => 1,
            Self::UnsupportedConstruct { .. } => 2,
            Self::MissingType { .. } => 3,
            Self::UnassignableType { .. } => 4,
            Self::InaccessibleSymbol { .. } => 5,
            Self::InconsistentFunctionOverride { .. } => 6,
            Self::NonCallable { .. } => 7,
            Self::NonIndexable { .. } => 8,
            Self::NonExhaustiveMatch { .. } => 9,
            Self::IncompletePattern { .. } => 10,
            Self::ConflictingPattern { .. } => 11,
            Self::MissingReturn { .. } => 12,
            Self::UninitializedVariable { .. } => 13,
            Self::IllegalCast { .. } => 14,
            Self::NoOverload { .. } => 15,
            Self::AmbiguousOverload { .. } => 16,
            Self::UnsupportedOperator { .. } => 17,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::UnsatisfiedDependency { dependency } => dependency.node(),
            Self::UnsupportedConstruct { node, .. } => *node,
            Self::MissingType { node, .. } => *node,
            Self::UnassignableType { node, .. } => *node,
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
            Self::NoOverload { node, .. } => *node,
            Self::AmbiguousOverload { node, .. } => *node,
            Self::UnsupportedOperator { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::UnsupportedConstruct { .. } => "unsupported construct".to_string(),
            Self::MissingType { .. } => "missing type".to_string(),
            Self::UnassignableType { .. } => "unassignable type".to_string(),
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
            Self::NoOverload { .. } => "no matching overload".to_string(),
            Self::AmbiguousOverload { .. } => "ambiguous overload".to_string(),
            Self::UnsupportedOperator { .. } => "unsupported operator for type".to_string(),
        }
    }
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzeError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Analyze.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<AnalyzeError> for TaskError {
    #[inline]
    fn from(error: AnalyzeError) -> Self {
        TaskError::Analyze(error)
    }
}

pub type AnalyzeResult<T> = Result<T, AnalyzeError>;
