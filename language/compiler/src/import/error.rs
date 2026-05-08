use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_parser::ParseError;
use destack_source::FileType;

/// Errors during source import.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Import)]
pub enum ImportError {
    // -------------------------------------------------------------------------
    // 1xx: Module / file resolution
    // -------------------------------------------------------------------------
    /// Module could not be resolved (filesystem or specifier resolution).
    #[diagnostic(code = "EI100", message = "module '{target}' not found")]
    ModuleNotFound {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Failed to parse a module.
    #[diagnostic(code = "EI101", message = "parse error")]
    ParseError {
        anchor: DiagnosticAnchor,
        diagnostics: Vec<ParseError>,
    },

    /// Failed to parse a data module.
    #[diagnostic(code = "EI102", message = "failed to parse {file_type}: {message}")]
    DataParseError {
        anchor: DiagnosticAnchor,
        file_type: FileType,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 3xx: Syntax / construct issues
    // -------------------------------------------------------------------------
    /// Unsupported construct in module (forbidden by spec).
    #[diagnostic(code = "EI300", message = "unsupported construct: {message}")]
    UnsupportedConstruct {
        anchor: DiagnosticAnchor,
        message: String,
    },

    /// Unsupported node.
    #[diagnostic(code = "EI301", message = "unsupported construct")]
    UnsupportedNode { anchor: DiagnosticAnchor },

    /// Directive prologue is invalid or cannot be interpreted.
    #[diagnostic(code = "EI303", message = "invalid directive prologue")]
    InvalidPrologue {
        anchor: DiagnosticAnchor,
        content: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal import failure.
    #[diagnostic(code = "EI900", message = "{message}")]
    Internal {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
