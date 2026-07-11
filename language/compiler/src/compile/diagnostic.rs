pub use destack_artifact::{DiagnosticAnchor, DiagnosticDefinition, DiagnosticFormat};

use crate::{
    AnalyzeError, AnalyzeWarning, BindError, BindWarning, CheckError, CheckWarning, EmitError,
    EmitWarning, ExpandError, ExpandWarning, ExportError, ExportWarning, ImportError,
    ImportWarning, LinkError, LinkWarning, LowerError, LowerWarning, MaterializeError,
    MaterializeWarning, OptimizeError, OptimizeWarning, VerifyError, VerifyWarning,
};
use destack_core::{NameMatch, NameMatchTier};
use destack_source::{Applicability, DiagnosticSuggestion, FilePatch, Patch, PatchSet};

/// Registry of all compiler diagnostic codes.
///
/// Provides compile-time access to all valid error and warning codes,
/// organized by phase. Use this for validation at CLI and test boundaries.
#[derive(Debug)]
pub struct DiagnosticRegistry;

impl DiagnosticRegistry {
    /// All error definitions from all phases.
    pub const ALL_ERRORS: &'static [&'static [DiagnosticDefinition]] = &[
        BindError::ALL,
        ImportError::ALL,
        ExpandError::ALL,
        ExportError::ALL,
        CheckError::ALL,
        MaterializeError::ALL,
        LowerError::ALL,
        VerifyError::ALL,
        AnalyzeError::ALL,
        OptimizeError::ALL,
        EmitError::ALL,
        LinkError::ALL,
    ];

    /// All warning definitions from all phases.
    pub const ALL_WARNINGS: &'static [&'static [DiagnosticDefinition]] = &[
        BindWarning::ALL,
        ImportWarning::ALL,
        ExpandWarning::ALL,
        ExportWarning::ALL,
        CheckWarning::ALL,
        MaterializeWarning::ALL,
        LowerWarning::ALL,
        VerifyWarning::ALL,
        AnalyzeWarning::ALL,
        OptimizeWarning::ALL,
        EmitWarning::ALL,
        LinkWarning::ALL,
    ];

    /// Check if an error code is valid.
    pub fn is_valid_error_code(code: &str) -> bool {
        BindError::is_valid_code(code)
            || ImportError::is_valid_code(code)
            || ExpandError::is_valid_code(code)
            || ExportError::is_valid_code(code)
            || CheckError::is_valid_code(code)
            || MaterializeError::is_valid_code(code)
            || LowerError::is_valid_code(code)
            || VerifyError::is_valid_code(code)
            || AnalyzeError::is_valid_code(code)
            || OptimizeError::is_valid_code(code)
            || EmitError::is_valid_code(code)
            || LinkError::is_valid_code(code)
    }

    /// Check if a warning code is valid.
    pub fn is_valid_warning_code(code: &str) -> bool {
        BindWarning::is_valid_code(code)
            || ImportWarning::is_valid_code(code)
            || ExpandWarning::is_valid_code(code)
            || ExportWarning::is_valid_code(code)
            || CheckWarning::is_valid_code(code)
            || MaterializeWarning::is_valid_code(code)
            || LowerWarning::is_valid_code(code)
            || VerifyWarning::is_valid_code(code)
            || AnalyzeWarning::is_valid_code(code)
            || OptimizeWarning::is_valid_code(code)
            || EmitWarning::is_valid_code(code)
            || LinkWarning::is_valid_code(code)
    }

    /// Check if a diagnostic code (error or warning) is valid.
    #[inline]
    pub fn is_valid_code(code: &str) -> bool {
        Self::is_valid_error_code(code) || Self::is_valid_warning_code(code)
    }

    /// Look up a diagnostic definition by code.
    pub fn definition(code: &str) -> Option<&'static DiagnosticDefinition> {
        BindError::definition(code)
            .or_else(|| ImportError::definition(code))
            .or_else(|| ExpandError::definition(code))
            .or_else(|| ExportError::definition(code))
            .or_else(|| CheckError::definition(code))
            .or_else(|| MaterializeError::definition(code))
            .or_else(|| LowerError::definition(code))
            .or_else(|| VerifyError::definition(code))
            .or_else(|| AnalyzeError::definition(code))
            .or_else(|| OptimizeError::definition(code))
            .or_else(|| EmitError::definition(code))
            .or_else(|| LinkError::definition(code))
            .or_else(|| BindWarning::definition(code))
            .or_else(|| ImportWarning::definition(code))
            .or_else(|| ExpandWarning::definition(code))
            .or_else(|| ExportWarning::definition(code))
            .or_else(|| CheckWarning::definition(code))
            .or_else(|| MaterializeWarning::definition(code))
            .or_else(|| LowerWarning::definition(code))
            .or_else(|| VerifyWarning::definition(code))
            .or_else(|| AnalyzeWarning::definition(code))
            .or_else(|| OptimizeWarning::definition(code))
            .or_else(|| EmitWarning::definition(code))
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
            'B' => BindError::ALL_CODES,
            'I' => ImportError::ALL_CODES,
            'X' => ExpandError::ALL_CODES,
            'T' => ExportError::ALL_CODES,
            'C' => CheckError::ALL_CODES,
            'M' => MaterializeError::ALL_CODES,
            'L' => LowerError::ALL_CODES,
            'V' => VerifyError::ALL_CODES,
            'A' => AnalyzeError::ALL_CODES,
            'O' => OptimizeError::ALL_CODES,
            'G' => EmitError::ALL_CODES,
            'K' => LinkError::ALL_CODES,
            _ => &[],
        }
    }

    /// Get all warning codes for a given phase letter.
    pub fn phase_warning_codes(letter: char) -> &'static [&'static str] {
        match letter {
            'B' => BindWarning::ALL_CODES,
            'I' => ImportWarning::ALL_CODES,
            'X' => ExpandWarning::ALL_CODES,
            'T' => ExportWarning::ALL_CODES,
            'C' => CheckWarning::ALL_CODES,
            'M' => MaterializeWarning::ALL_CODES,
            'L' => LowerWarning::ALL_CODES,
            'V' => &[],
            'A' => AnalyzeWarning::ALL_CODES,
            'O' => OptimizeWarning::ALL_CODES,
            'G' => EmitWarning::ALL_CODES,
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

/// Return the maximum edit distance for one diagnostic suggestion.
pub(crate) fn diagnostic_suggestion_distance(value: &str) -> usize {
    (value.chars().count() / 3).max(1)
}

/// Return the rename suggestion for one matched misspelled name.
pub(crate) fn rename_suggestion(
    anchor: &DiagnosticAnchor,
    best: &NameMatch<String>,
) -> Option<DiagnosticSuggestion> {
    let DiagnosticAnchor::Span(span) = anchor else {
        return None;
    };

    // only a unique same-name match applies without review
    let applicability = match (best.tier, best.is_unique) {
        (NameMatchTier::Case | NameMatchTier::Separators, true) => Applicability::Automatic,
        _ => Applicability::Dangerous,
    };
    let patches = PatchSet::from_files(vec![FilePatch {
        file: span.file,
        patches: vec![Patch::replace(*span, best.candidate.clone())],
    }]);

    Some(DiagnosticSuggestion::new(
        format!("rename to '{}'", best.candidate),
        patches,
        applicability,
    ))
}
