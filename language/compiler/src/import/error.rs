use std::path::PathBuf;

use dyst_dir::{GlobalNodeIdAny, ModuleId, Program};
use dyst_parser::ParseError;
use dyst_source::{StringId, Uri};

use crate::{CompileError, CompilePhase, CompileTaskDependency};

/// Error when importing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ImportError {
    /// Wait for task dependency.
    Yield { wait: CompileTaskDependency },
    /// Invalid URI.
    InvalidUri { uri: Uri },
    /// File URI not found.
    FileUriNotFound { uri: Uri },
    /// File not found.
    FilePathNotFound { path: PathBuf },
    /// Module could not be resolved.
    ModuleNotFound {
        target: StringId,
        module: ModuleId,
        error: Option<dyst_resolver::ResolveError>,
    },
    /// Failed to parse a module.
    ParseError {
        module: ModuleId,
        node: GlobalNodeIdAny,
        diagnostics: Vec<ParseError>,
    },
    /// Circular dependency.
    CircularDependency { node: GlobalNodeIdAny },
}

impl ImportError {
    /// Get the numeric sub-code of the error (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::InvalidUri { .. } => 1,
            Self::FileUriNotFound { .. } => 2,
            Self::FilePathNotFound { .. } => 3,
            Self::ModuleNotFound { .. } => 4,
            Self::ParseError { .. } => 5,
            Self::CircularDependency { .. } => 6,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Yield { wait } => wait.first_node(),
            Self::InvalidUri { .. } => None,
            Self::FileUriNotFound { .. } => None,
            Self::FilePathNotFound { .. } => None,
            Self::ModuleNotFound { .. } => None,
            Self::ParseError { node, .. } => Some(*node),
            Self::CircularDependency { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Yield { .. } => "unresolved dependency".to_string(),
            Self::InvalidUri { uri } => format!("invalid URI: '{uri}'"),
            Self::FileUriNotFound { uri } => format!("file URI not found: '{uri}'"),
            Self::FilePathNotFound { path } => format!("file path not found: '{path:?}'"),
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
                &format!("E{}{:03}", CompilePhase::Import.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type ImportResult<T> = Result<T, ImportError>;

impl From<ImportError> for CompileError {
    #[inline]
    fn from(error: ImportError) -> Self {
        CompileError::Import(error)
    }
}
