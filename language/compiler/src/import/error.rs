use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_dir::GlobalScopeId;
use destack_parser::ParseError;
use destack_source::{FileType, ModuleId};

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
    // 2xx: Binding / export conflicts
    // -------------------------------------------------------------------------
    /// Conflicting symbol binding (unambiguous).
    #[diagnostic(code = "EI200", message = "duplicate identifier {name}")]
    ConflictingBinding {
        anchor: DiagnosticAnchor,
        other: DiagnosticAnchor,
        scope: GlobalScopeId,
        name: String,
        is_local: bool,
    },

    /// Conflicting export name in the same module.
    #[diagnostic(code = "EI201", message = "duplicate export {name}")]
    ConflictingExport {
        anchor: DiagnosticAnchor,
        other: DiagnosticAnchor,
        module: ModuleId,
        name: String,
    },

    /// Conflicting default export.
    #[diagnostic(code = "EI202", message = "duplicate default export '{name}'")]
    ConflictingDefaultExport {
        anchor: DiagnosticAnchor,
        other: DiagnosticAnchor,
        name: String,
        module: ModuleId,
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

    /// Reserved identifier used in a forbidden context.
    #[diagnostic(code = "EI302", message = "reserved identifier '{name}'")]
    ReservedIdentifier {
        anchor: DiagnosticAnchor,
        name: String,
    },

    /// Directive prologue is invalid or cannot be interpreted.
    #[diagnostic(code = "EI303", message = "invalid directive prologue")]
    InvalidPrologue {
        anchor: DiagnosticAnchor,
        content: String,
    },

    /// Import declarations must be direct module roots.
    #[diagnostic(code = "EI304", message = "import declarations must be top-level")]
    ImportNotTopLevel { anchor: DiagnosticAnchor },

    /// Export declarations must be direct module roots.
    #[diagnostic(code = "EI305", message = "export declarations must be top-level")]
    ExportNotTopLevel { anchor: DiagnosticAnchor },

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
