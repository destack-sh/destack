use crate::{BuildError, EvaluateError, ExecuteError, LoadError, OptimizeError, ValidateError};

/// Compiler diagnostic that can be turned into a CompileError.
#[allow(dead_code)]
pub trait CompileDiagnostic {
    /// Get the numeric sub-code of the error.
    fn sub_code(&self) -> u8;

    /// Get the family code of the error.
    fn family_letter(&self) -> &'static str;

    /// Get the family number of the error.
    fn family_number(&self) -> u8;

    /// Get the full code of the error.
    #[inline]
    fn full_code(&self) -> String {
        format!("{}{:03}", self.family_letter(), self.sub_code())
    }
}

/// Error during compilation.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum CompileError {
    /// Error during loading (code `L`).
    Load(LoadError) = 1,
    /// Error during evaluation (code `E`).
    Evaluate(EvaluateError) = 2,
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
            Self::Load(_) => "L",
            Self::Evaluate(_) => "E",
            Self::Validate(_) => "V",
            Self::Execute(_) => "X",
            Self::Optimize(_) => "O",
            Self::Build(_) => "B",
        }
    }

    /// Get the family number of the error (e.g., `1` for `L001` / `1001`).
    pub fn family_number(&self) -> u8 {
        match self {
            Self::Load(_) => 1,
            Self::Evaluate(_) => 2,
            Self::Validate(_) => 3,
            Self::Execute(_) => 4,
            Self::Optimize(_) => 5,
            Self::Build(_) => 6,
        }
    }

    /// Get the numeric sub-code of the error (e.g., `1` for `L001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Load(error) => error.sub_code(),
            Self::Evaluate(error) => error.sub_code(),
            Self::Validate(error) => error.sub_code(),
            Self::Execute(error) => error.sub_code(),
            Self::Optimize(error) => error.sub_code(),
            Self::Build(error) => error.sub_code(),
        }
    }

    /// Get the full code of the error (e.g., `L001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("{}E{:03}", self.family_letter(), self.sub_code())
    }

    /// Get the numeric code of the error (e.g., `1001` for `L001`).
    #[inline]
    pub fn numeric_code(&self) -> u16 {
        (self.family_number() as u16) * 1000 + self.sub_code() as u16
    }
}

pub type CompileResult<T> = Result<T, CompileError>;
