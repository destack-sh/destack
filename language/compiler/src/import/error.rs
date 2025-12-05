use destack_dir::{GlobalNodeIdAny};
use destack_parser::ParseError;
use destack_source::StringId;

use crate::{TaskDependency, TaskError, TaskPhase};

use destack_workspace::Program;

/// Error when importing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ImportError {
    /// Module could not be resolved.
    ModuleNotFound {
        node: GlobalNodeIdAny,
        target: StringId,
        error: Option<destack_resolver::ResolveError>,
    },
    /// Failed to parse a module.
    ParseError {
        node: GlobalNodeIdAny,
        diagnostics: Vec<ParseError>,
    },
}

impl TryFrom<ImportError> for TaskDependency {
    type Error = ImportError;

    fn try_from(error: ImportError) -> Result<Self, Self::Error> {
        // interface required for tasks but imports cannot yield
        Err(error)
    }
}

impl ImportError {
    /// Get the numeric sub-code of the error (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ModuleNotFound { .. } => 2,
            Self::ParseError { .. } => 3,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::ModuleNotFound { node, .. } => *node,
            Self::ParseError { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::ModuleNotFound { target, .. } => {
                let target_str = program.strings.get(*target).to_string();
                format!("module '{target_str}' not found")
            }
            Self::ParseError { .. } => "parse error".to_string(),
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Import.letter(), self.sub_code()),
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
