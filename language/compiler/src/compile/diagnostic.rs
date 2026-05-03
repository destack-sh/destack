pub use destack_artifact::{DiagnosticAnchor, DiagnosticDefinition, DiagnosticFormat};

use crate::emit::{EmitError, EmitWarning};
use crate::{
    AnalyzeError, AnalyzeWarning, ElaborateError, ElaborateWarning, ExecuteError, ExecuteWarning,
    GenerateError, GenerateWarning, ImportError, ImportWarning, LinkError, LinkWarning, LowerError,
    LowerWarning, OptimizeError, OptimizeWarning, ResolveError, ResolveWarning,
};

/// Registry of all compiler diagnostic codes.
///
/// Provides compile-time access to all valid error and warning codes,
/// organized by phase. Use this for validation at CLI and test boundaries.
#[derive(Debug)]
pub struct DiagnosticRegistry;

impl DiagnosticRegistry {
    /// All error definitions from all phases.
    pub const ALL_ERRORS: &'static [&'static [DiagnosticDefinition]] = &[
        ImportError::ALL,
        ResolveError::ALL,
        AnalyzeError::ALL,
        ElaborateError::ALL,
        ExecuteError::ALL,
        LowerError::ALL,
        OptimizeError::ALL,
        GenerateError::ALL,
        LinkError::ALL,
        EmitError::ALL,
    ];

    /// All warning definitions from all phases.
    pub const ALL_WARNINGS: &'static [&'static [DiagnosticDefinition]] = &[
        ImportWarning::ALL,
        ResolveWarning::ALL,
        AnalyzeWarning::ALL,
        ElaborateWarning::ALL,
        ExecuteWarning::ALL,
        LowerWarning::ALL,
        OptimizeWarning::ALL,
        GenerateWarning::ALL,
        LinkWarning::ALL,
        EmitWarning::ALL,
    ];

    /// Check if an error code is valid.
    pub fn is_valid_error_code(code: &str) -> bool {
        ImportError::is_valid_code(code)
            || ResolveError::is_valid_code(code)
            || AnalyzeError::is_valid_code(code)
            || ElaborateError::is_valid_code(code)
            || ExecuteError::is_valid_code(code)
            || LowerError::is_valid_code(code)
            || OptimizeError::is_valid_code(code)
            || GenerateError::is_valid_code(code)
            || LinkError::is_valid_code(code)
            || EmitError::is_valid_code(code)
    }

    /// Check if a warning code is valid.
    pub fn is_valid_warning_code(code: &str) -> bool {
        ImportWarning::is_valid_code(code)
            || ResolveWarning::is_valid_code(code)
            || AnalyzeWarning::is_valid_code(code)
            || ElaborateWarning::is_valid_code(code)
            || ExecuteWarning::is_valid_code(code)
            || LowerWarning::is_valid_code(code)
            || OptimizeWarning::is_valid_code(code)
            || GenerateWarning::is_valid_code(code)
            || LinkWarning::is_valid_code(code)
            || EmitWarning::is_valid_code(code)
    }

    /// Check if a diagnostic code (error or warning) is valid.
    #[inline]
    pub fn is_valid_code(code: &str) -> bool {
        Self::is_valid_error_code(code) || Self::is_valid_warning_code(code)
    }

    /// Look up a diagnostic definition by code.
    pub fn definition(code: &str) -> Option<&'static DiagnosticDefinition> {
        ImportError::definition(code)
            .or_else(|| ResolveError::definition(code))
            .or_else(|| AnalyzeError::definition(code))
            .or_else(|| ElaborateError::definition(code))
            .or_else(|| ExecuteError::definition(code))
            .or_else(|| LowerError::definition(code))
            .or_else(|| OptimizeError::definition(code))
            .or_else(|| GenerateError::definition(code))
            .or_else(|| LinkError::definition(code))
            .or_else(|| EmitError::definition(code))
            .or_else(|| ImportWarning::definition(code))
            .or_else(|| ResolveWarning::definition(code))
            .or_else(|| AnalyzeWarning::definition(code))
            .or_else(|| ElaborateWarning::definition(code))
            .or_else(|| ExecuteWarning::definition(code))
            .or_else(|| LowerWarning::definition(code))
            .or_else(|| OptimizeWarning::definition(code))
            .or_else(|| GenerateWarning::definition(code))
            .or_else(|| LinkWarning::definition(code))
            .or_else(|| EmitWarning::definition(code))
    }

    /// Look up a diagnostic definition by variant name.
    pub fn named_definition(name: &str) -> Option<&'static DiagnosticDefinition> {
        let matches_name = |definition: &&DiagnosticDefinition| definition.name == name;

        Self::ALL_ERRORS
            .iter()
            .flat_map(|defs| defs.iter())
            .find(matches_name)
            .or_else(|| {
                Self::ALL_WARNINGS
                    .iter()
                    .flat_map(|defs| defs.iter())
                    .find(matches_name)
            })
    }

    /// Get all error codes for a given phase letter.
    pub fn phase_error_codes(letter: char) -> &'static [&'static str] {
        match letter {
            'I' => ImportError::ALL_CODES,
            'R' => ResolveError::ALL_CODES,
            'A' => AnalyzeError::ALL_CODES,
            'E' => ElaborateError::ALL_CODES,
            'X' => ExecuteError::ALL_CODES,
            'M' => LowerError::ALL_CODES,
            'O' => OptimizeError::ALL_CODES,
            'G' => GenerateError::ALL_CODES,
            'K' => LinkError::ALL_CODES,
            'W' => EmitError::ALL_CODES,
            _ => &[],
        }
    }

    /// Get all warning codes for a given phase letter.
    pub fn phase_warning_codes(letter: char) -> &'static [&'static str] {
        match letter {
            'I' => ImportWarning::ALL_CODES,
            'R' => ResolveWarning::ALL_CODES,
            'A' => AnalyzeWarning::ALL_CODES,
            'E' => ElaborateWarning::ALL_CODES,
            'X' => ExecuteWarning::ALL_CODES,
            'M' => LowerWarning::ALL_CODES,
            'O' => OptimizeWarning::ALL_CODES,
            'G' => GenerateWarning::ALL_CODES,
            'K' => LinkWarning::ALL_CODES,
            'W' => EmitWarning::ALL_CODES,
            _ => &[],
        }
    }

    /// Validate a list of codes and return any invalid ones.
    pub fn validate_codes(codes: &[String]) -> Vec<&str> {
        codes
            .iter()
            .filter(|c| !Self::is_valid_code(c))
            .map(|c| c.as_str())
            .collect()
    }
}
