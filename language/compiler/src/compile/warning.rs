use crate::{
    BuildWarning, ResolveWarning, ExecuteWarning, ImportWarning, OptimizeWarning, ValidateWarning,
};

/// Warning during compilation.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum CompileWarning {
    /// Warning during importing (code `I`).
    Import(ImportWarning) = 1,
    /// Warning during evaluation (code `E`).
    Resolve(ResolveWarning) = 2,
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
            Self::Import(_) => "I",
            Self::Resolve(_) => "E",
            Self::Validate(_) => "V",
            Self::Execute(_) => "X",
            Self::Optimize(_) => "O",
            Self::Build(_) => "B",
        }
    }

    /// Get the numeric sub-code of the warning (e.g., `1` for `IE001`).
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Import(warning) => warning.sub_code(),
            Self::Resolve(warning) => warning.sub_code(),
            Self::Validate(warning) => warning.sub_code(),
            Self::Execute(warning) => warning.sub_code(),
            Self::Optimize(warning) => warning.sub_code(),
            Self::Build(warning) => warning.sub_code(),
        }
    }

    /// Get the full code of the error (e.g., `IE001`).
    #[inline]
    pub fn full_code(&self) -> String {
        format!("{}W{:03}", self.family_letter(), self.sub_code())
    }
}

