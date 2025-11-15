use dyst_dir::Session;
use dyst_source::{Diagnostic, DiagnosticSeverity};

use crate::TranspileError;

/// Diagnostic encountered during transpilation.
#[derive(Debug, Clone)]
pub enum TranspileDiagnostic {
    /// Error.
    Error(TranspileError),
}

impl TranspileDiagnostic {
    /// Get the severity of the diagnostic.
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            Self::Error(_) => DiagnosticSeverity::Error,
        }
    }

    /// Turn the diagnostic into a full Dyst diagnostic.
    pub fn to_diagnostic<'a>(&self, session: &'a Session<'a>) -> Diagnostic {
        match self {
            Self::Error(error) => error.to_diagnostic(session),
        }
    }
}

pub type TranspileResult<T> = Result<T, TranspileError>;
