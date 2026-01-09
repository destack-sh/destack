use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_ast::StringId;
use destack_compiler_macros::DefineError;
use destack_dir::{GlobalNodeIdAny, GlobalScopeId, StaticKey};
use destack_parser::ParseError;
use destack_source::{ModuleId, Span};
use destack_workspace::Program;

/// Errors during the import phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Import)]
pub enum ImportError {
    /// Yield to a dependency.
    #[error(code = "EI000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Unsatisfied dependency (dependency failed).
    #[error(code = "EI001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Module could not be resolved (filesystem or specifier resolution).
    #[error(code = "EI002", message = "module '{target}' not found")]
    ModuleNotFound {
        target: StringId,
        error: Option<destack_resolver::ResolveError>,
    },

    /// Failed to parse a module.
    #[error(code = "EI003", message = "parse error")]
    ParseError {
        node: GlobalNodeIdAny,
        diagnostics: Vec<ParseError>,
    },

    /// Unsupported construct in module (forbidden by spec).
    #[error(code = "EI004", message = "unsupported construct: {message}")]
    UnsupportedConstruct { span: Span, message: String },

    /// Unsupported node.
    #[error(code = "EI005", message = "unsupported construct")]
    UnsupportedNode { node: GlobalNodeIdAny },

    /// Conflicting symbol binding (unambiguous).
    #[error(code = "EI006", message = "duplicate identifier '{name}'")]
    ConflictingBinding {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        name: Option<StaticKey>,
    },

    /// Conflicting export name in the same module.
    #[error(code = "EI007", message = "duplicate export '{name}'")]
    ConflictingExport {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        module: ModuleId,
        name: Option<StaticKey>,
    },

    /// Conflicting default export.
    #[error(code = "EI008", message = "duplicate default export '{name}'")]
    ConflictingDefaultExport {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        name: Option<StringId>,
        module: ModuleId,
    },

    /// Reserved identifier used in a forbidden context.
    #[error(code = "EI009", message = "reserved identifier '{name}'")]
    ReservedIdentifier {
        node: GlobalNodeIdAny,
        name: StringId,
    },

    /// Directive prologue is invalid or cannot be interpreted.
    #[error(code = "EI010", message = "invalid directive prologue")]
    InvalidPrologue {
        node: GlobalNodeIdAny,
        content: String,
    },
}
