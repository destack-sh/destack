use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
    TaskSkipReason,
};
use destack_ast::StringId;
use destack_compiler_macros::DefineError;
use destack_dir::{AnchoredGlobalNodeId, GlobalScopeId, StaticKey};
use destack_parser::ParseError;
use destack_source::{FileType, ModuleId, Span};
use destack_workspace::Program;

/// Errors during the import phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Import)]
pub enum ImportError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / dependency
    // -------------------------------------------------------------------------
    /// Yield to a dependency.
    #[error(code = "EI000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Unsatisfied dependency (dependency failed).
    #[error(code = "EI001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Task was skipped due to stale versions.
    #[error(code = "EI002", message = "task skipped")]
    Skipped { reason: TaskSkipReason },

    // -------------------------------------------------------------------------
    // 1xx: Module / file resolution
    // -------------------------------------------------------------------------
    /// Module could not be resolved (filesystem or specifier resolution).
    #[error(code = "EI100", message = "module '{target}' not found")]
    ModuleNotFound {
        target: StringId,
        error: Option<destack_resolver::ResolveError>,
    },

    /// Failed to parse a module.
    #[error(code = "EI101", message = "parse error")]
    ParseError {
        node: AnchoredGlobalNodeId,
        diagnostics: Vec<ParseError>,
    },

    /// Failed to parse a data module.
    #[error(code = "EI102", message = "failed to parse {file_type}: {message}")]
    DataParseError {
        span: Span,
        file_type: FileType,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 2xx: Binding / export conflicts
    // -------------------------------------------------------------------------
    /// Conflicting symbol binding (unambiguous).
    #[error(code = "EI200", message = "duplicate identifier '{name}'")]
    ConflictingBinding {
        node: AnchoredGlobalNodeId,
        other_node: AnchoredGlobalNodeId,
        scope: GlobalScopeId,
        name: Option<StaticKey>,
        is_local: bool,
    },

    /// Conflicting export name in the same module.
    #[error(code = "EI201", message = "duplicate export '{name}'")]
    ConflictingExport {
        node: AnchoredGlobalNodeId,
        other_node: AnchoredGlobalNodeId,
        module: ModuleId,
        name: Option<StaticKey>,
    },

    /// Conflicting default export.
    #[error(code = "EI202", message = "duplicate default export '{name}'")]
    ConflictingDefaultExport {
        node: AnchoredGlobalNodeId,
        other_node: AnchoredGlobalNodeId,
        name: Option<StringId>,
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 3xx: Syntax / construct issues
    // -------------------------------------------------------------------------
    /// Unsupported construct in module (forbidden by spec).
    #[error(code = "EI300", message = "unsupported construct: {message}")]
    UnsupportedConstruct { span: Span, message: String },

    /// Unsupported node.
    #[error(code = "EI301", message = "unsupported construct")]
    UnsupportedNode { node: AnchoredGlobalNodeId },

    /// Reserved identifier used in a forbidden context.
    #[error(code = "EI302", message = "reserved identifier '{name}'")]
    ReservedIdentifier {
        node: AnchoredGlobalNodeId,
        name: StringId,
    },

    /// Directive prologue is invalid or cannot be interpreted.
    #[error(code = "EI303", message = "invalid directive prologue")]
    InvalidPrologue {
        node: AnchoredGlobalNodeId,
        content: String,
    },
}
