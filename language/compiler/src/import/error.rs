use dyst_dir::{GlobalNodeIdAny, Program};
use dyst_parser::ParseError;
use dyst_source::StringId;

use crate::{Phase, TaskDependency, TaskError};

/// Error when importing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ImportError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    YieldFailed { dependency: TaskDependency },
    /// Module could not be resolved.
    ModuleNotFound {
        node: GlobalNodeIdAny,
        target: StringId,
        error: Option<dyst_resolver::ResolveError>,
    },
    /// Failed to parse a module.
    ParseError {
        node: GlobalNodeIdAny,
        diagnostics: Vec<ParseError>,
    },
    /// Circular dependency.
    CircularDependency { node: GlobalNodeIdAny },
}

impl TryFrom<ImportError> for TaskDependency {
    type Error = ImportError;

    fn try_from(error: ImportError) -> Result<Self, Self::Error> {
        match error {
            ImportError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl ImportError {
    /// Get the numeric sub-code of the error (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::YieldFailed { .. } => 1,
            Self::ModuleNotFound { .. } => 2,
            Self::ParseError { .. } => 3,
            Self::CircularDependency { .. } => 4,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::YieldFailed { dependency } => dependency.node(),
            Self::ModuleNotFound { node, .. } => *node,
            Self::ParseError { node, .. } => *node,
            Self::CircularDependency { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::YieldFailed { .. } => "unsatisfied dependency".to_string(),
            Self::ModuleNotFound { target, .. } => {
                let target_str = program.strings.get(*target).to_string();
                format!("module '{target_str}' not found")
            }
            Self::ParseError { .. } => "parse error".to_string(),
            Self::CircularDependency { .. } => "circular dependency".to_string(),
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportError")
            .field(
                "code",
                &format!("E{}{:03}", Phase::Import.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type ImportResult<T> = Result<T, ImportError>;

impl From<ImportError> for TaskError {
    #[inline]
    fn from(error: ImportError) -> Self {
        TaskError::Import(error)
    }
}
