use dyst_dir::ModuleId;
use dyst_parser::ParserError;
use dyst_source::Uri;

use crate::{CompilerDiagnostic, CompilerError};

/// Error when loading something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum LoadError {
    /// File not found on disk.
    FileNotFound { path: Uri } = 1,
    /// Failed to parse a file.
    ParseError {
        module_id: ModuleId,
        diagnostics: Vec<ParserError>,
    } = 2,
    /// Circular dependency.
    CircularDependency { module_id: ModuleId } = 3,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadError")
            .field("code", &self.full_code())
            .finish()
    }
}

pub type LoadResult<T> = Result<T, LoadError>;

impl From<LoadError> for CompilerError {
    #[inline]
    fn from(error: LoadError) -> Self {
        CompilerError::Load(error)
    }
}

impl CompilerDiagnostic for LoadError {
    #[inline]
    fn family_letter(&self) -> &'static str {
        "L"
    }

    #[inline]
    fn family_number(&self) -> u8 {
        1
    }

    #[inline]
    fn sub_code(&self) -> u8 {
        self.sub_code()
    }
}

impl LoadError {
    /// Get the numeric sub-code of the error.
    #[inline]
    fn sub_code(&self) -> u8 {
        match self {
            Self::FileNotFound { .. } => 1,
            Self::ParseError { .. } => 2,
            Self::CircularDependency { .. } => 3,
        }
    }
}
