use std::borrow::Cow;

use tspp_artifact::{
    DiagnosticAnchor, DiagnosticContext, DiagnosticError, DiagnosticRecord, ToDiagnostic,
};
use tspp_source::{Diagnostic, DiagnosticSeverity};

use crate::Lint;

/// A diagnostic emitted by one lint.
#[derive(Debug, Clone, PartialEq)]
pub struct LinterDiagnostic {
    /// The lint id.
    pub(crate) id: Cow<'static, str>,
    /// The severity.
    pub(crate) severity: Option<DiagnosticSeverity>,
    /// The message.
    pub(crate) message: String,
    /// The primary anchor.
    pub(crate) primary: DiagnosticAnchor,
}

impl LinterDiagnostic {
    /// Create a lint diagnostic.
    pub(crate) fn new(
        lint: &Lint,
        message: impl Into<String>,
        primary: impl Into<DiagnosticAnchor>,
    ) -> Self {
        Self {
            id: lint.id.clone(),
            severity: None,
            message: message.into(),
            primary: primary.into(),
        }
    }

    /// Set the severity.
    pub(crate) fn set_severity(&mut self, severity: Option<DiagnosticSeverity>) {
        self.severity = severity;
    }
}

impl LinterDiagnostic {
    /// Convert the lint diagnostic into a stored diagnostic record.
    pub fn to_record(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<DiagnosticRecord, DiagnosticError> {
        Ok(DiagnosticRecord::new(self.to_diagnostic(context)?))
    }
}

impl ToDiagnostic for LinterDiagnostic {
    /// Convert the lint diagnostic into a source diagnostic.
    fn to_diagnostic(
        &self,
        context: &dyn DiagnosticContext,
    ) -> Result<Diagnostic, DiagnosticError> {
        let severity = self
            .severity
            .ok_or_else(|| DiagnosticError::InvalidDiagnostic {
                message: format!("lint '{}' has no diagnostic severity", self.id),
            })?;
        let primary = context.label(&self.primary, None)?;

        Ok(Diagnostic::new(
            self.id.to_string(),
            severity,
            self.message.clone(),
            primary,
        ))
    }
}
