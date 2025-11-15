use crate::{
    BuildError, ResolveError, ExecuteError, ImportError, OptimizeError, ValidateError,
};

/// Error during compilation.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum CompileError {
    /// Error during importing (code `I`).
    Import(ImportError) = 1,
    /// Error during evaluation (code `E`).
    Resolve(ResolveError) = 2,
    /// Error during validation (code `V`).
    Validate(ValidateError) = 3,
    /// Error during execution (code `X`).
    Execute(ExecuteError) = 4,
    /// Error during optimization (code `O`).
    Optimize(OptimizeError) = 5,
    /// Error during building (code `B`).
    Build(BuildError) = 6,
}

impl CompileError {
    /// Get the family letter of the error.
    pub fn family_letter(&self) -> &str {
        match self {
            Self::Import(_) => "I",
            Self::Resolve(_) => "E",
            Self::Validate(_) => "V",
            Self::Execute(_) => "X",
            Self::Optimize(_) => "O",
            Self::Build(_) => "B",
        }
    }

    /// Get the numeric sub-code of the error (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Import(error) => error.sub_code(),
            Self::Resolve(error) => error.sub_code(),
            Self::Validate(error) => error.sub_code(),
            Self::Execute(error) => error.sub_code(),
            Self::Optimize(error) => error.sub_code(),
            Self::Build(error) => error.sub_code(),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("{}E{:03}", self.family_letter(), self.sub_code())
    }
}

pub type CompileResult<T> = Result<T, CompileError>;

