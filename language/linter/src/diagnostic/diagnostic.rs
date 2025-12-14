use destack_source::{Diagnostic, DiagnosticSeverity, FileId, LabeledSpan, Span, Suggestion};
use destack_workspace::{LintCategory, LintSeverity};

use super::LintFix;

/// A lint diagnostic produced by a lint rule.
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    /// The lint rule ID (e.g., "no-floating-promise").
    pub rule_id: &'static str,
    /// The lint code (e.g., "LC001").
    pub code: &'static str,
    /// The category of the lint.
    pub category: LintCategory,
    /// The configured severity.
    pub severity: LintSeverity,
    /// The diagnostic message.
    pub message: String,
    /// The file containing the issue.
    pub file_id: FileId,
    /// The primary span of the issue.
    pub span: Span,
    /// Label for the primary span.
    pub label: String,
    /// Secondary spans with labels.
    pub secondary_spans: Vec<LabeledSpan>,
    /// Suggested fixes.
    pub fixes: Vec<LintFix>,
    /// Help text or notes.
    pub notes: Vec<String>,
}

impl LintDiagnostic {
    /// Create a new lint diagnostic.
    pub fn new(
        rule_id: &'static str,
        code: &'static str,
        category: LintCategory,
        severity: LintSeverity,
        message: impl Into<String>,
        file_id: FileId,
        span: Span,
    ) -> Self {
        Self {
            rule_id,
            code,
            category,
            severity,
            message: message.into(),
            file_id,
            span,
            label: String::new(),
            secondary_spans: Vec::new(),
            fixes: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Add a label to the primary span.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add a secondary span.
    pub fn with_secondary(mut self, span: LabeledSpan) -> Self {
        self.secondary_spans.push(span);
        self
    }

    /// Add a fix.
    pub fn with_fix(mut self, fix: LintFix) -> Self {
        self.fixes.push(fix);
        self
    }

    /// Add a note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Convert to a standard Diagnostic.
    pub fn into_diagnostic(self) -> Diagnostic {
        let severity = match self.severity {
            // shouldn't happen, but treat as note
            LintSeverity::Off => return self.into_diagnostic_as(DiagnosticSeverity::Note),
            LintSeverity::Note => DiagnosticSeverity::Note,
            LintSeverity::Warning => DiagnosticSeverity::Warning,
            LintSeverity::Error => DiagnosticSeverity::Error,
        };
        self.into_diagnostic_as(severity)
    }

    /// Convert to a standard Diagnostic with a specific severity.
    fn into_diagnostic_as(self, severity: DiagnosticSeverity) -> Diagnostic {
        let primary_span = LabeledSpan {
            span: self.span,
            label: self.label,
        };

        let suggestions: Option<Vec<Suggestion>> = if self.fixes.is_empty() {
            None
        } else {
            Some(
                self.fixes
                    .into_iter()
                    .map(|f| f.into_suggestion())
                    .collect(),
            )
        };

        Diagnostic {
            code: self.code.to_string(),
            original_code: None,
            severity,
            original_severity: None,
            message: self.message,
            file_id: self.file_id,
            primary_span,
            primary_highlight_spans: None,
            secondary_spans: if self.secondary_spans.is_empty() {
                None
            } else {
                Some(self.secondary_spans)
            },
            suggestions,
        }
    }

    /// Check if this diagnostic should be reported based on severity.
    pub fn is_enabled(&self) -> bool {
        self.severity != LintSeverity::Off
    }
}
