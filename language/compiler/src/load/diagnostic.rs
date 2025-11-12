use dyst_dir::ModuleId;
use dyst_parser::ParserError;
use dyst_source::{FileId, Uri};

use dyst_source::StringId;

use crate::{CompilerDiagnostic, CompilerError};

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
        diagnostics: Vec<ParserError>,
    } = 4,
    /// Circular dependency.
    CircularDependency { module_id: ModuleId } = 5,
}

impl LoadError {
    /// Get the numeric sub-code of the error.
    #[inline]
    fn sub_code(&self) -> u8 {
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
