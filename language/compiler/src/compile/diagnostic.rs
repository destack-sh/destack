use crate::{CompileError, CompileWarning};
use dyst_dir::{GlobalNodeIdAny, Session};
use dyst_source::{Diagnostic, DiagnosticSeverity, LabeledSpan};

/// Diagnostic encountered during compilation.
#[derive(Debug, Clone)]
pub enum CompileDiagnostic {
    /// Error.
    Error(CompileError),
    /// Warning.
    Warning(CompileWarning),
}

impl From<CompileError> for CompileDiagnostic {
    fn from(error: CompileError) -> Self {
        Self::Error(error)
    }
}

impl From<CompileWarning> for CompileDiagnostic {
    fn from(warning: CompileWarning) -> Self {
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
    pub fn message<'a>(&self, session: &'a Session<'a>) -> String {
        match self {
            Self::Error(error) => error.message(session),
            Self::Warning(warning) => warning.message(session),
        }
    }

    /// Get the node id of the diagnostic.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Error(error) => error.node_id(),
            Self::Warning(warning) => warning.node_id(),
        }
    }

    /// Get the full code of the diagnostic.
    pub fn full_code(&self) -> String {
        match self {
            Self::Error(error) => error.full_code(),
            Self::Warning(warning) => warning.full_code(),
        }
    }

    /// Turn the diagnostic into a full Dyst diagnostic.
    pub fn to_diagnostic<'a>(&self, session: &'a Session<'a>) -> Diagnostic {
        // get source information
        let node_id = self
            .node_id()
            .unwrap_or_else(|| panic!("TODO #Broken: diagnostic without node id"));
        let module = session
            .modules
            .get(node_id.module_id)
            .unwrap_or_else(|| panic!("module not found: {:?}", node_id.module_id));
        let file_id = module.read().file;

        // make diagnostic
        let severity = self.severity();
        let message = self.message(session);
        let code = self.full_code();
        let primary_span = module.read().ast.get_span_by_id(node_id.local_id.id);
        let primary_span = LabeledSpan {
            span: primary_span,
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
