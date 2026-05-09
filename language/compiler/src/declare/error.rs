use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_parser::ParseError;
use destack_source::FileType;

/// Errors during source declaration.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Declare)]
pub enum DeclareError {
    // -------------------------------------------------------------------------
    // 1xx: Module / file resolution
    // -------------------------------------------------------------------------
    /// Module could not be resolved (filesystem or specifier resolution).
    #[diagnostic(code = "ED100", message = "module '{target}' not found")]
    ModuleNotFound {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Failed to parse a module.
    #[diagnostic(code = "ED101", message = "parse error")]
    ParseError {
        anchor: DiagnosticAnchor,
        diagnostics: Vec<ParseError>,
    },

    /// Failed to parse a data module.
    #[diagnostic(code = "ED102", message = "failed to parse {file_type}: {message}")]
    DataParseError {
        anchor: DiagnosticAnchor,
        file_type: FileType,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 3xx: Syntax / construct issues
    // -------------------------------------------------------------------------
    /// Unsupported construct in module (forbidden by spec).
    #[diagnostic(code = "ED300", message = "unsupported construct: {message}")]
    UnsupportedConstruct {
        anchor: DiagnosticAnchor,
        message: String,
    },

    /// Unsupported node.
    #[diagnostic(code = "ED301", message = "unsupported construct")]
    UnsupportedNode { anchor: DiagnosticAnchor },

    /// Directive prologue is invalid or cannot be interpreted.
    #[diagnostic(code = "ED303", message = "invalid directive prologue")]
    InvalidPrologue {
        anchor: DiagnosticAnchor,
        content: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal declaration failure.
    #[diagnostic(code = "ED900", message = "{message}")]
    Internal {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
