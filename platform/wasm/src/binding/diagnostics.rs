use destack_source as source;
use serde::{Deserialize, Serialize};

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSeverity {
    /// Informational note.
    Note,
    /// Warning diagnostic.
    Warning,
    /// Error diagnostic.
    Error,
}

impl From<source::DiagnosticSeverity> for DiagnosticSeverity {
    fn from(severity: source::DiagnosticSeverity) -> Self {
        match severity {
            source::DiagnosticSeverity::Note => DiagnosticSeverity::Note,
            source::DiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
            source::DiagnosticSeverity::Error => DiagnosticSeverity::Error,
        }
    }
}

/// Span payload for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    /// File id for this span.
    pub file_id: u32,
    /// Span start offset.
    pub start: u32,
    /// Span end offset.
    pub end: u32,
}

impl From<source::Span> for Span {
    fn from(span: source::Span) -> Self {
        Self {
            file_id: span.file.0,
            start: span.start,
            end: span.end,
        }
    }
}

/// Labeled span payload for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabeledSpan {
    /// Source span payload.
    pub span: Span,
    /// Label for this span.
    pub label: String,
}

impl From<source::LabeledSpan> for LabeledSpan {
    fn from(span: source::LabeledSpan) -> Self {
        Self {
            span: span.span.into(),
            label: span.label,
        }
    }
}

/// Suggestion style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionStyle {
    /// Standard suggestion display.
    Normal,
    /// Short suggestion display.
    Short,
    /// Hidden suggestion display.
    Hidden,
    /// Verbose suggestion display.
    Verbose,
}

impl From<source::SuggestionStyle> for SuggestionStyle {
    fn from(style: source::SuggestionStyle) -> Self {
        match style {
            source::SuggestionStyle::Normal => SuggestionStyle::Normal,
            source::SuggestionStyle::Short => SuggestionStyle::Short,
            source::SuggestionStyle::Hidden => SuggestionStyle::Hidden,
            source::SuggestionStyle::Verbose => SuggestionStyle::Verbose,
        }
    }
}

/// Suggestion applicability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Applicability {
    /// The suggestion can be applied automatically.
    Automatic,
    /// The suggestion may be unsafe to apply automatically.
    Dangerous,
}

impl From<source::Applicability> for Applicability {
    fn from(applicability: source::Applicability) -> Self {
        match applicability {
            source::Applicability::Automatic => Applicability::Automatic,
            source::Applicability::Dangerous => Applicability::Dangerous,
        }
    }
}

/// Suggestion payload for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    /// Spans involved in the suggestion.
    pub spans: Vec<LabeledSpan>,
    /// Replacement text when available.
    pub replacement: Option<String>,
    /// Suggestion message.
    pub message: String,
    /// Suggestion style.
    pub style: SuggestionStyle,
    /// Suggestion applicability.
    pub applicability: Applicability,
}

impl From<source::Suggestion> for Suggestion {
    fn from(suggestion: source::Suggestion) -> Self {
        Self {
            spans: suggestion.spans.into_iter().map(Into::into).collect(),
            replacement: suggestion.replacement,
            message: suggestion.message,
            style: suggestion.style.into(),
            applicability: suggestion.applicability.into(),
        }
    }
}

/// Shared diagnostic payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Original diagnostic code before mapping.
    pub original_code: Option<String>,
    /// Diagnostic severity.
    pub severity: DiagnosticSeverity,
    /// Original diagnostic severity before mapping.
    pub original_severity: Option<DiagnosticSeverity>,
    /// Diagnostic message.
    pub message: String,
    /// Primary file id for this diagnostic.
    pub file_id: u32,
    /// Primary labeled span.
    pub primary_span: LabeledSpan,
    /// Primary highlight spans.
    pub primary_highlight_spans: Option<Vec<LabeledSpan>>,
    /// Secondary spans.
    pub secondary_spans: Option<Vec<LabeledSpan>>,
    /// Suggestions for this diagnostic.
    pub suggestions: Option<Vec<Suggestion>>,
}

impl From<source::Diagnostic> for Diagnostic {
    fn from(diagnostic: source::Diagnostic) -> Self {
        Self {
            code: diagnostic.code,
            original_code: diagnostic.original_code,
            severity: diagnostic.severity.into(),
            original_severity: diagnostic.original_severity.map(Into::into),
            message: diagnostic.message,
            file_id: diagnostic.file_id.0,
            primary_span: diagnostic.primary_span.into(),
            primary_highlight_spans: diagnostic
                .primary_highlight_spans
                .map(|spans| spans.into_iter().map(Into::into).collect()),
            secondary_spans: diagnostic
                .secondary_spans
                .map(|spans| spans.into_iter().map(Into::into).collect()),
            suggestions: diagnostic
                .suggestions
                .map(|suggestions| suggestions.into_iter().map(Into::into).collect()),
        }
    }
}

/// Diagnostic options for re-mapping errors and warnings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticOptions {
    /// Which warning codes to error on (as errors).
    pub error_warnings: Vec<String>,
    /// Which error codes to suppress (as warnings).
    pub suppress_errors: Vec<String>,
    /// Which warning codes to suppress.
    pub suppress_warnings: Vec<String>,
}

impl From<DiagnosticOptions> for source::DiagnosticOptions {
    fn from(options: DiagnosticOptions) -> Self {
        Self {
            error_warnings: options.error_warnings,
            suppress_errors: options.suppress_errors,
            suppress_warnings: options.suppress_warnings,
        }
    }
}

/// Return true when diagnostics include at least one error.
pub fn diagnostics_have_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
}

/// Return true when diagnostics include at least one warning.
pub fn diagnostics_have_warnings(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
}
