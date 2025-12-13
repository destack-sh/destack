use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskError};
use destack_compiler_macros::DefineError;
use destack_dir::GlobalNodeIdAny;
use destack_parser::ParseError;
use destack_source::{Span, StringId};
use destack_workspace::Program;

/// Errors during the import phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Import)]
pub enum ImportError {
    /// Module could not be resolved (filesystem or specifier resolution).
    #[error(code = "EI001", message = "module not found")]
    ModuleNotFound {
        target: StringId,
        error: Option<destack_resolver::ResolveError>,
    },

    /// Failed to parse a module.
    #[error(code = "EI002", message = "parse error")]
    ParseError {
        node: GlobalNodeIdAny,
        diagnostics: Vec<ParseError>,
    },

    /// Unsupported construct in module (forbidden by spec).
    #[error(code = "EI003", message = "unsupported construct")]
    UnsupportedConstruct { span: Span, message: String },
}
