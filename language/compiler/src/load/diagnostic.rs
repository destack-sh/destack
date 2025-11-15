use dyst_dir::ModuleId;
use dyst_parser::ParseError;
use dyst_source::{FileId, Uri};

use dyst_source::StringId;

use crate::CompileError;

/// Error when loading something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum LoadError {
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

impl LoadError {
    /// Get the numeric sub-code of the error (e.g., `1` for `LE001`).
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

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadError")
            .field("code", &format!("LE{:03}", self.sub_code()))
            .finish()
    }
}

pub type LoadResult<T> = Result<T, LoadError>;

impl From<LoadError> for CompileError {
    #[inline]
    fn from(error: LoadError) -> Self {
        CompileError::Load(error)
    }
}

/// Warning when loading something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LoadWarning {
    /// Missing configuration for a file.
    MissingConfiguration { module: ModuleId } = 1,
}

impl LoadWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingConfiguration { .. } => 1,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::MissingConfiguration { .. } => "missing configuration for a module",
        }
    }
}

impl std::fmt::Display for LoadWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadWarning")
            .field("code", &format!("LW{:03}", self.sub_code()))
            .finish()
    }
}
