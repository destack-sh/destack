use dyst_dir::ModuleId;
use dyst_parser::ParseError;
use dyst_source::{FileId, StringId, Uri};

use crate::CompileError;

/// Error when importing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ImportError {
    /// File ID not found.
    FileIdNotFound { file_id: FileId } = 1,
    /// File URI not found.
    FileUriNotFound { uri: Uri } = 2,
    /// Module not found.
    ModuleNotFound { target: StringId } = 3,
    /// Failed to parse a module.
    ParseError {
        module_id: ModuleId,
        diagnostics: Vec<ParseError>,
    } = 4,
    /// Circular dependency.
    CircularDependency { module_id: ModuleId } = 5,
}

impl ImportError {
    /// Get the numeric sub-code of the error (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::FileIdNotFound { .. } => 1,
            Self::FileUriNotFound { .. } => 2,
            Self::ModuleNotFound { .. } => 3,
            Self::ParseError { .. } => 4,
            Self::CircularDependency { .. } => 5,
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportError")
            .field("code", &format!("LE{:03}", self.sub_code()))
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

