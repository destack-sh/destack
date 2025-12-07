use destack_dir::GlobalNodeIdAny;

use crate::{TaskDependency, TaskDependencyError, TaskError, TaskPhase};

use destack_workspace::Program;

/// Error during code generation.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum GenerateError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Unsupported target/output format.
    UnsupportedTarget {
        node: GlobalNodeIdAny,
        target: String,
    },
    /// Unsupported construct (instruction, expression, etc.).
    UnsupportedConstruct { node: GlobalNodeIdAny },
    /// Unsupported type for codegen.
    UnsupportedType { node: GlobalNodeIdAny },
    /// Unexpected construct (wrong node type).
    UnexpectedConstruct { node: GlobalNodeIdAny },
    /// Unresolved construct (not fully resolved before codegen).
    UnresolvedConstruct { node: GlobalNodeIdAny },
    /// Unresolved function reference.
    UnresolvedFunction { node: GlobalNodeIdAny, name: String },
    /// Missing type information.
    MissingType { node: GlobalNodeIdAny },
    /// Internal codegen error.
    Internal {
        node: GlobalNodeIdAny,
        message: String,
    },
}

impl From<TaskDependencyError> for GenerateError {
    fn from(e: TaskDependencyError) -> Self {
        match e {
            TaskDependencyError::NotReady { dependency } => Self::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                Self::UnsatisfiedDependency { dependency }
            }
        }
    }
}

impl TryFrom<GenerateError> for TaskDependency {
    type Error = GenerateError;

    fn try_from(error: GenerateError) -> Result<Self, Self::Error> {
        match error {
            GenerateError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

pub type GenerateResult<T> = Result<T, GenerateError>;

impl From<GenerateError> for TaskError {
    #[inline]
    fn from(error: GenerateError) -> Self {
        TaskError::Generate(error)
    }
}

impl GenerateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsatisfiedDependency { .. } => 1,
            Self::UnsupportedTarget { .. } => 2,
            Self::UnsupportedConstruct { .. } => 3,
            Self::UnsupportedType { .. } => 4,
            Self::UnexpectedConstruct { .. } => 5,
            Self::UnresolvedConstruct { .. } => 6,
            Self::UnresolvedFunction { .. } => 7,
            Self::MissingType { .. } => 8,
            Self::Internal { .. } => 9,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::UnsatisfiedDependency { dependency } => dependency.node(),
            Self::UnsupportedTarget { node, .. } => *node,
            Self::UnsupportedConstruct { node, .. } => *node,
            Self::UnsupportedType { node, .. } => *node,
            Self::UnexpectedConstruct { node, .. } => *node,
            Self::UnresolvedConstruct { node, .. } => *node,
            Self::UnresolvedFunction { node, .. } => *node,
            Self::MissingType { node, .. } => *node,
            Self::Internal { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::UnsupportedTarget { target, .. } => {
                format!("unsupported target: {target}")
            }
            Self::UnsupportedConstruct { .. } => "unsupported construct".to_string(),
            Self::UnsupportedType { .. } => "unsupported type".to_string(),
            Self::UnexpectedConstruct { .. } => "unexpected construct".to_string(),
            Self::UnresolvedConstruct { .. } => "unresolved construct".to_string(),
            Self::UnresolvedFunction { name, .. } => {
                format!("unresolved function: {name}")
            }
            Self::MissingType { .. } => "missing type".to_string(),
            Self::Internal { message, .. } => format!("internal error: {message}"),
        }
    }
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerateError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Generate.letter(), self.sub_code()),
            )
            .finish()
    }
}
