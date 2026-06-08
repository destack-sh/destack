use destack_artifact::{DiagnosticAnchor, DiagnosticContext, DiagnosticError, ToDiagnostic};
use destack_repository::{LintCategory, LintSeverity};
use destack_source::{
    Diagnostic, DiagnosticHelp, DiagnosticNote, DiagnosticSeverity, LabeledSpan, Span,
};

use super::LintFix;

/// A lint report produced by one lint rule.
#[derive(Debug, Clone)]
pub struct LintReport {
    /// The lint rule ID (e.g., "no-floating-promise").
    pub rule_id: &'static str,
    /// The stable diagnostic code.
    pub code: &'static str,
    /// The category of the lint.
    pub category: LintCategory,
    /// The configured severity.
    pub severity: LintSeverity,
    /// The primary diagnostic message.
    pub message: String,
    /// The primary source span.
    pub primary: Span,
    /// The optional primary label message.
    pub label: Option<String>,
    /// Additional source labels.
    pub labels: Vec<LabeledSpan>,
    /// Extra context for understanding the diagnostic.
    pub notes: Vec<DiagnosticNote>,
    /// Guidance for fixing or avoiding the diagnostic.
    pub helps: Vec<DiagnosticHelp>,
    /// Suggested fixes.
    pub fixes: Vec<LintFix>,
}

impl LintReport {
    /// Create a new lint report.
    pub fn new(
        rule_id: &'static str,
        code: &'static str,
        category: LintCategory,
        severity: LintSeverity,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            rule_id,
            code,
            category,
            severity,
            message: message.into(),
            primary: span,
            label: None,
            labels: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            fixes: Vec::new(),
        }
    }

    /// Add a label to the primary span.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());

        self
    }

    /// Add a secondary span.
    pub fn secondary(mut self, span: LabeledSpan) -> Self {
        self.labels.push(span);

        self
    }

    /// Add a fix.
    pub fn fix(mut self, fix: LintFix) -> Self {
        self.fixes.push(fix);

        self
    }

    /// Add a note.
    pub fn note(mut self, note: impl Into<DiagnosticNote>) -> Self {
        self.notes.push(note.into());

        self
    }

    /// Add a help message.
    pub fn help(mut self, help: impl Into<DiagnosticHelp>) -> Self {
        self.helps.push(help.into());

        self
    }

    /// Return the diagnostic code.
    pub fn code(&self) -> &str {
        self.code
    }

    /// Return the diagnostic message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the primary label message.
    pub fn label_message(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Iterate over diagnostic notes.
    pub fn notes(&self) -> impl Iterator<Item = &str> {
        self.notes.iter().map(|note| note.message.as_str())
    }

    /// Return true when the diagnostic has no suggestions.
    pub fn has_no_fixes(&self) -> bool {
        self.fixes.is_empty()
    }

    /// Check if this report should be emitted based on severity.
    pub fn is_enabled(&self) -> bool {
        self.severity != LintSeverity::Off
    }

    /// Return the final diagnostic severity.
    pub fn diagnostic_severity(&self) -> DiagnosticSeverity {
        match self.severity {
            LintSeverity::Off => DiagnosticSeverity::Note,
            LintSeverity::Note => DiagnosticSeverity::Note,
            LintSeverity::Warning => DiagnosticSeverity::Warning,
            LintSeverity::Error => DiagnosticSeverity::Error,
        }
    }
}

impl ToDiagnostic for LintReport {
    /// Convert this lint report into one final diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<Diagnostic, DiagnosticError> {
        let primary = context.label(&DiagnosticAnchor::Span(self.primary), self.label.clone())?;
        let mut diagnostic = Diagnostic::new(
            self.code,
            self.diagnostic_severity(),
            self.message.clone(),
            primary,
        );

        for label in &self.labels {
            let label = context.label(
                &DiagnosticAnchor::Span(label.span),
                Some(label.label.clone()),
            )?;
            diagnostic = diagnostic.label(label);
        }

        for note in &self.notes {
            diagnostic = diagnostic.note(note.clone());
        }

        for help in &self.helps {
            diagnostic = diagnostic.help(help.clone());
        }

        for fix in &self.fixes {
            let suggestion = fix.to_suggestion(context)?;
            diagnostic = diagnostic.suggestion(suggestion);
        }

        Ok(diagnostic)
    }
}
