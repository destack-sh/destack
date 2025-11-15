use crate::{
    BuildError, BuildWarning, EvaluateError, EvaluateWarning, ExecuteError, ExecuteWarning,
    LoadError, LoadWarning, OptimizeError, OptimizeWarning, ValidateError, ValidateWarning,
};

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

    /// Get the numeric sub-code of the error (e.g., `1` for `LE001`).
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

    /// Get the full code of the error (e.g., `LE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("{}E{:03}", self.family_letter(), self.sub_code())
    }
}

pub type CompileResult<T> = Result<T, CompileError>;

/// Warning during compilation.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum CompileWarning {
    /// Warning during loading (code `L`).
    Load(LoadWarning) = 1,
    /// Warning during evaluation (code `E`).
    Evaluate(EvaluateWarning) = 2,
    /// Warning during validation (code `V`).
    Validate(ValidateWarning) = 3,
    /// Warning during execution (code `X`).
    Execute(ExecuteWarning) = 4,
    /// Warning during optimization (code `O`).
    Optimize(OptimizeWarning) = 5,
    /// Warning during building (code `B`).
    Build(BuildWarning) = 6,
}

impl CompileWarning {
    /// Get the family letter of the warning.
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

    /// Get the numeric sub-code of the warning (e.g., `1` for `LE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Load(warning) => warning.sub_code(),
            Self::Evaluate(warning) => warning.sub_code(),
            Self::Validate(warning) => warning.sub_code(),
            Self::Execute(warning) => warning.sub_code(),
            Self::Optimize(warning) => warning.sub_code(),
            Self::Build(warning) => warning.sub_code(),
        }
    }

    /// Get the full code of the error (e.g., `LE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("{}W{:03}", self.family_letter(), self.sub_code())
    }
}
