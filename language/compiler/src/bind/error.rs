use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_ast::StringId;
use destack_compiler_macros::DefineError;
use destack_dir::{GlobalNodeIdAny, GlobalScopeId, StaticKey};
use destack_source::ModuleId;
use destack_workspace::Program;

/// Errors during the bind phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Bind)]
pub enum BindError {
    /// Yield to a dependency.
    #[error(code = "EB000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Unsatisfied dependency (dependency failed).
    #[error(code = "EB001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Unsupported node.
    #[error(code = "EB002", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },

    /// Conflicting symbol binding (unambiguous).
    #[error(code = "EB003", message = "conflicting binding")]
    ConflictingBinding {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        name: Option<StaticKey>,
    },

    /// Conflicting export name in the same module.
    #[error(code = "EB004", message = "conflicting export")]
    ConflictingExport {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        module: ModuleId,
        name: Option<StaticKey>,
    },

    /// Conflicting default export.
    #[error(code = "EB005", message = "conflicting default export")]
    ConflictingDefaultExport {
        node: GlobalNodeIdAny,
        other_node: GlobalNodeIdAny,
        name: Option<StringId>,
        module: ModuleId,
    },

    /// Reserved identifier used in a forbidden context.
    #[error(code = "EB006", message = "reserved identifier '{name}'")]
    ReservedIdentifier {
        node: GlobalNodeIdAny,
        name: StringId,
    },

    /// Directive prologue is invalid or cannot be interpreted.
    #[error(code = "EB007", message = "invalid directive prologue")]
    InvalidPrologue {
        node: GlobalNodeIdAny,
        content: String,
    },
}
