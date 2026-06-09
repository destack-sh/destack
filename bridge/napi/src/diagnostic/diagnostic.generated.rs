// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{BatchEdit, FileContentId, Span};

/// One concrete source label in a diagnostic.
#[derive(Debug)]
#[napi(object)]
pub struct DiagnosticLabel {
    /// Exact file content containing the span.
    pub content: FileContentId,
    /// Concrete source span.
    pub span: Span,
    /// Optional label shown on the span.
    pub message: Option<String>,
}

impl DiagnosticLabel {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticLabel) -> Self {
        Self {
            content: FileContentId::from_bridge(value.content),
            span: Span::from_bridge(value.span),
            message: value.message,
        }
    }
}

/// Extra context for understanding a diagnostic.
#[derive(Debug)]
#[napi(object)]
pub struct DiagnosticNote {
    /// Note message.
    pub message: String,
}

impl DiagnosticNote {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticNote) -> Self {
        Self {
            message: value.message,
        }
    }
}

/// Guidance for fixing or avoiding a diagnostic.
#[derive(Debug)]
#[napi(object)]
pub struct DiagnosticHelp {
    /// Help message.
    pub message: String,
}

impl DiagnosticHelp {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticHelp) -> Self {
        Self {
            message: value.message,
        }
    }
}

/// One suggested source change for a diagnostic.
#[derive(Debug)]
#[napi(object)]
pub struct DiagnosticSuggestion {
    /// Exact source edits for machine application.
    pub edits: BatchEdit,
    /// Source labels to show with the suggestion.
    pub labels: Vec<DiagnosticLabel>,
    /// Suggestion message.
    pub message: String,
    /// Suggestion applicability.
    pub applicability: String,
}

impl DiagnosticSuggestion {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::DiagnosticSuggestion) -> Self {
        Self {
            edits: BatchEdit::from_bridge(value.edits),
            labels: value
                .labels
                .into_iter()
                .map(|item| DiagnosticLabel::from_bridge(item))
                .collect(),
            message: value.message,
            applicability: applicability_label(value.applicability),
        }
    }
}

/// One final renderable diagnostic.
#[derive(Debug)]
#[napi(object)]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Diagnostic severity.
    pub severity: String,
    /// Diagnostic message.
    pub message: String,
    /// Main source label.
    pub primary: DiagnosticLabel,
    /// Additional source labels.
    pub labels: Vec<DiagnosticLabel>,
    /// Extra context.
    pub notes: Vec<DiagnosticNote>,
    /// Fixing or avoidance guidance.
    pub helps: Vec<DiagnosticHelp>,
    /// Suggested source changes.
    pub suggestions: Vec<DiagnosticSuggestion>,
    /// Extra semantic tags.
    pub tags: Vec<String>,
}

impl Diagnostic {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Diagnostic) -> Self {
        Self {
            code: value.code,
            severity: diagnostic_severity_label(value.severity),
            message: value.message,
            primary: DiagnosticLabel::from_bridge(value.primary),
            labels: value
                .labels
                .into_iter()
                .map(|item| DiagnosticLabel::from_bridge(item))
                .collect(),
            notes: value
                .notes
                .into_iter()
                .map(|item| DiagnosticNote::from_bridge(item))
                .collect(),
            helps: value
                .helps
                .into_iter()
                .map(|item| DiagnosticHelp::from_bridge(item))
                .collect(),
            suggestions: value
                .suggestions
                .into_iter()
                .map(|item| DiagnosticSuggestion::from_bridge(item))
                .collect(),
            tags: value
                .tags
                .into_iter()
                .map(|item| diagnostic_tag_label(item))
                .collect(),
        }
    }
}

/// Return one target enum label.
fn diagnostic_severity_label(value: bridge::DiagnosticSeverity) -> String {
    let label = match value {
        bridge::DiagnosticSeverity::Note => "note",
        bridge::DiagnosticSeverity::Warning => "warning",
        bridge::DiagnosticSeverity::Error => "error",
    };
    label.to_string()
}

/// Return one target enum label.
fn diagnostic_tag_label(value: bridge::DiagnosticTag) -> String {
    let label = match value {
        bridge::DiagnosticTag::Unnecessary => "unnecessary",
        bridge::DiagnosticTag::Deprecated => "deprecated",
    };
    label.to_string()
}

/// Return one target enum label.
fn applicability_label(value: bridge::Applicability) -> String {
    let label = match value {
        bridge::Applicability::Automatic => "automatic",
        bridge::Applicability::Unsafe => "unsafe",
        bridge::Applicability::Dangerous => "dangerous",
    };
    label.to_string()
}
