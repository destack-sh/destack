pub use tspp_artifact::{DiagnosticAnchor, DiagnosticFormat};
pub use tspp_source::DiagnosticDefinition;

use crate::{
    AnalyzeError, AnalyzeWarning, BindError, BindWarning, CheckError, CheckWarning, Compiler,
    ElaborateError, ElaborateWarning, EmitError, EmitWarning, ExpandError, ExpandWarning,
    ExportError, ExportWarning, ImportError, InstantiateError, LinkError, LinkWarning, LowerError,
    LowerWarning, MaterializeError, MaterializeWarning, ResolveError, ResolveWarning, VerifyError,
};
use tspp_core::{NameMatch, NameMatchTier};
use tspp_source::{Applicability, DiagnosticSuggestion, FilePatch, Patch, PatchSet};

/// All compiler diagnostic definitions.
const COMPILER_DIAGNOSTICS: &[&[DiagnosticDefinition]] = &[
    BindError::ALL,
    BindWarning::ALL,
    ImportError::ALL,
    ExpandError::ALL,
    ExpandWarning::ALL,
    ExportError::ALL,
    ExportWarning::ALL,
    ResolveError::ALL,
    ResolveWarning::ALL,
    CheckError::ALL,
    CheckWarning::ALL,
    MaterializeError::ALL,
    MaterializeWarning::ALL,
    LowerError::ALL,
    LowerWarning::ALL,
    VerifyError::ALL,
    ElaborateError::ALL,
    ElaborateWarning::ALL,
    InstantiateError::ALL,
    AnalyzeError::ALL,
    AnalyzeWarning::ALL,
    EmitError::ALL,
    EmitWarning::ALL,
    LinkError::ALL,
    LinkWarning::ALL,
];

impl Compiler {
    /// Iterate every compiler diagnostic definition.
    pub fn diagnostic_definitions() -> impl Iterator<Item = &'static DiagnosticDefinition> {
        COMPILER_DIAGNOSTICS
            .iter()
            .flat_map(|definitions| definitions.iter())
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
