use dyst_dir::{NodeIdAny, NodeTree, Session};
use dyst_source::{Diagnostic, DiagnosticSeverity, LabeledSpan};

use crate::{TranspileError, TranspileWarning};

/// Diagnostic encountered during transpilation.
#[derive(Debug, Clone)]
pub enum TranspileDiagnostic {
    /// Error.
    Error(TranspileError),
    /// Warning.
    Warning(TranspileWarning),
}

impl TranspileDiagnostic {
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
    pub fn node_id(&self) -> NodeIdAny {
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
    pub fn to_diagnostic<'a>(&self, session: &'a Session<'a>, tree: &NodeTree) -> Diagnostic {
        // get source information
        let node_id = self.node_id();
        let (module_id, ast_id) = tree.get_source(node_id.id);
        let module = session
            .modules
            .get(module_id)
            .unwrap_or_else(|| panic!("module not found: {module_id:?}"));
        let file_id = module.read().file;
        let file = session
            .files
            .get(file_id)
            .unwrap_or_else(|| panic!("file not found: {file_id:?}"));

        // make diagnostic
        let severity = self.severity();
        let message = self.message(session);
        let code = self.full_code();
        let primary_span = ast_id
            .map(|ast_id| module.read().ast.get_span_by_id(ast_id))
            .unwrap_or_else(|| file.span());
        let primary_span = LabeledSpan {
            span: primary_span,
            label: message.clone(),
        };

        Diagnostic {
            code,
            severity,
            original_severity: None,
            message,
            file_id,
            primary_span,
            secondary_spans: None,
            suggestions: None,
        }
    }
}
