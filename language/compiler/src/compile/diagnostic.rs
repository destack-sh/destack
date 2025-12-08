use crate::{DiagnosticAnchor, TaskError, TaskWarning};
use destack_source::{Diagnostic, DiagnosticSeverity, LabeledSpan, Span};

use destack_workspace::Program;

/// Diagnostic encountered during compilation.
#[derive(Debug, Clone)]
pub enum CompileDiagnostic {
    /// Error.
    Error(TaskError),
    /// Warning.
    Warning(TaskWarning),
}

impl From<TaskError> for CompileDiagnostic {
    fn from(error: TaskError) -> Self {
        Self::Error(error)
    }
}

impl From<TaskWarning> for CompileDiagnostic {
    fn from(warning: TaskWarning) -> Self {
        Self::Warning(warning)
    }
}

impl CompileDiagnostic {
    /// Get the severity of the diagnostic.
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            Self::Error(_) => DiagnosticSeverity::Error,
            Self::Warning(_) => DiagnosticSeverity::Warning,
        }
    }

    /// Get the message of the diagnostic.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Error(error) => error.message(program),
            Self::Warning(warning) => warning.message(program),
        }
    }

    /// Get the anchor of the diagnostic.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Error(error) => error.anchor(),
            Self::Warning(warning) => warning.anchor(),
        }
    }

    /// Get the full code of the diagnostic.
    pub fn full_code(&self) -> String {
        match self {
            Self::Error(error) => error.full_code(),
            Self::Warning(warning) => warning.full_code(),
        }
    }

    /// Turn the diagnostic into a full Destack diagnostic.
    pub fn to_diagnostic(&self, program: &Program) -> Diagnostic {
        let anchor = self.anchor();
        let severity = self.severity();
        let message = self.message(program);
        let code = self.full_code();

        // get file and span from anchor, falling back to program's fallback file
        let (file_id, span) = anchor.to_file_span(program).unwrap_or_else(|| {
            let fallback = program.fallback_file_id;
            (fallback, Span::empty(fallback))
        });

        let primary_span = LabeledSpan {
            span,
            label: message.clone(),
        };

        Diagnostic {
            code,
            original_code: None,
            severity,
            original_severity: None,
            message,
            file_id,
            primary_span,
            primary_highlight_spans: None,
            secondary_spans: None,
            suggestions: None,
        }
    }
}
