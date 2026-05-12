pub use destack_artifact::{DiagnosticAnchor, DiagnosticDefinition, DiagnosticFormat};

use crate::{
    CheckError, CheckWarning, DeclareError, DeclareWarning, ElaborateError, ElaborateWarning,
    ExpandError, ExpandWarning, ExportError, ExportWarning, GenerateError, GenerateWarning,
    ImportError, ImportWarning, LinkError, LinkWarning, LowerError, LowerWarning, MaterializeError,
    MaterializeWarning, OptimizeError, OptimizeWarning, VerifyError,
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
        DeclareError::ALL,
        ImportError::ALL,
        ExpandError::ALL,
        ExportError::ALL,
        CheckError::ALL,
        ElaborateError::ALL,
        MaterializeError::ALL,
        LowerError::ALL,
        VerifyError::ALL,
        OptimizeError::ALL,
        GenerateError::ALL,
        LinkError::ALL,
    ];

    /// All warning definitions from all phases.
    pub const ALL_WARNINGS: &'static [&'static [DiagnosticDefinition]] = &[
        DeclareWarning::ALL,
        ImportWarning::ALL,
        ExpandWarning::ALL,
        ExportWarning::ALL,
        CheckWarning::ALL,
        ElaborateWarning::ALL,
        MaterializeWarning::ALL,
        LowerWarning::ALL,
        VerifyWarning::ALL,
        OptimizeWarning::ALL,
        GenerateWarning::ALL,
        LinkWarning::ALL,
    ];

    /// Check if an error code is valid.
    pub fn is_valid_error_code(code: &str) -> bool {
        DeclareError::is_valid_code(code)
            || ImportError::is_valid_code(code)
            || ExpandError::is_valid_code(code)
            || ExportError::is_valid_code(code)
            || CheckError::is_valid_code(code)
            || ElaborateError::is_valid_code(code)
            || MaterializeError::is_valid_code(code)
            || LowerError::is_valid_code(code)
            || VerifyError::is_valid_code(code)
            || OptimizeError::is_valid_code(code)
            || GenerateError::is_valid_code(code)
            || LinkError::is_valid_code(code)
    }

    /// Check if a warning code is valid.
    pub fn is_valid_warning_code(code: &str) -> bool {
        DeclareWarning::is_valid_code(code)
            || ImportWarning::is_valid_code(code)
            || ExpandWarning::is_valid_code(code)
            || ExportWarning::is_valid_code(code)
            || CheckWarning::is_valid_code(code)
            || ElaborateWarning::is_valid_code(code)
            || MaterializeWarning::is_valid_code(code)
            || LowerWarning::is_valid_code(code)
            || VerifyWarning::is_valid_code(code)
            || OptimizeWarning::is_valid_code(code)
            || GenerateWarning::is_valid_code(code)
            || LinkWarning::is_valid_code(code)
    }

    /// Check if a diagnostic code (error or warning) is valid.
    #[inline]
    pub fn is_valid_code(code: &str) -> bool {
        Self::is_valid_error_code(code) || Self::is_valid_warning_code(code)
    }

    /// Look up a diagnostic definition by code.
    pub fn definition(code: &str) -> Option<&'static DiagnosticDefinition> {
        DeclareError::definition(code)
            .or_else(|| ImportError::definition(code))
            .or_else(|| ExpandError::definition(code))
            .or_else(|| ExportError::definition(code))
            .or_else(|| CheckError::definition(code))
            .or_else(|| ElaborateError::definition(code))
            .or_else(|| MaterializeError::definition(code))
            .or_else(|| LowerError::definition(code))
            .or_else(|| VerifyError::definition(code))
            .or_else(|| OptimizeError::definition(code))
            .or_else(|| GenerateError::definition(code))
            .or_else(|| LinkError::definition(code))
            .or_else(|| DeclareWarning::definition(code))
            .or_else(|| ImportWarning::definition(code))
            .or_else(|| ExpandWarning::definition(code))
            .or_else(|| ExportWarning::definition(code))
            .or_else(|| CheckWarning::definition(code))
            .or_else(|| ElaborateWarning::definition(code))
            .or_else(|| MaterializeWarning::definition(code))
            .or_else(|| LowerWarning::definition(code))
            .or_else(|| VerifyWarning::definition(code))
            .or_else(|| OptimizeWarning::definition(code))
            .or_else(|| GenerateWarning::definition(code))
            .or_else(|| LinkWarning::definition(code))
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
            'D' => DeclareError::ALL_CODES,
            'I' => ImportError::ALL_CODES,
            'X' => ExpandError::ALL_CODES,
            'T' => ExportError::ALL_CODES,
            'C' => CheckError::ALL_CODES,
            'E' => ElaborateError::ALL_CODES,
            'M' => MaterializeError::ALL_CODES,
            'L' => LowerError::ALL_CODES,
            'V' => VerifyError::ALL_CODES,
            'O' => OptimizeError::ALL_CODES,
            'G' => GenerateError::ALL_CODES,
            'K' => LinkError::ALL_CODES,
            _ => &[],
        }
    }

    /// Get all warning codes for a given phase letter.
    pub fn phase_warning_codes(letter: char) -> &'static [&'static str] {
        match letter {
            'D' => DeclareWarning::ALL_CODES,
            'I' => ImportWarning::ALL_CODES,
            'X' => ExpandWarning::ALL_CODES,
            'T' => ExportWarning::ALL_CODES,
            'C' => CheckWarning::ALL_CODES,
            'E' => ElaborateWarning::ALL_CODES,
            'M' => MaterializeWarning::ALL_CODES,
            'L' => LowerWarning::ALL_CODES,
            'V' => &[],
            'O' => OptimizeWarning::ALL_CODES,
            'G' => GenerateWarning::ALL_CODES,
            'K' => LinkWarning::ALL_CODES,
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
