use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_builtin::LanguageItem;
use destack_compiler_macros::DefineError;
use destack_dir::{GlobalNodeIdAny, GlobalScopeId, GlobalSymbolId, StaticKey, StringId};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{Program, TargetId};

/// Errors during the resolve phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Resolve)]
pub enum ResolveError {
    /// Wait for task dependency.
    #[error(code = "ER000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "ER001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Unsupported node.
    #[error(code = "ER002", message = "unsupported {node}")]
    UnsupportedConstruct { node: GlobalNodeIdAny },

    /// Circular dependency.
    #[error(code = "ER003", message = "circular dependency")]
    CircularDependency {
        node: GlobalNodeIdAny,
        depends_on: Vec<GlobalNodeIdAny>,
    },

    /// Use of undeclared symbol.
    #[error(code = "ER004", message = "missing symbol {key}")]
    UndeclaredSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        key: StaticKey,
    },

    /// Use of missing symbol.
    #[error(code = "ER005", message = "missing symbol {key}")]
    MissingSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        via_module: Option<ModuleId>,
        key: StaticKey,
    },

    /// Use of ambiguous symbol.
    #[error(code = "ER006", message = "ambiguous symbol {key}")]
    AmbiguousSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        key: StaticKey,
    },

    /// Unresolved module.
    #[error(code = "ER007", message = "unresolved module '{target}'")]
    UnresolvedModule {
        node: GlobalNodeIdAny,
        target: StringId,
    },

    /// Cyclic symbol reference (re-export chain forms a cycle).
    #[error(code = "ER008", message = "cyclic reference to '{symbol}'")]
    CyclicSymbol {
        node: GlobalNodeIdAny,
        symbol: GlobalSymbolId,
    },

    /// Missing target for a control flow expression.
    #[error(code = "ER010", message = "missing target")]
    MissingTarget {
        node: GlobalNodeIdAny,
        target: Option<StringId>,
    },

    /// Invalid target for a control flow expression.
    #[error(code = "ER011", message = "invalid target")]
    InvalidTarget {
        node: GlobalNodeIdAny,
        target: Option<StringId>,
        target_node: GlobalNodeIdAny,
    },

    /// Missing language item (builtin not found).
    #[error(code = "ER012", message = "missing language item '{item}'")]
    MissingLanguageItem { item: LanguageItem },

    /// Missing builtin library.
    #[error(code = "ER013", message = "missing builtin lib '{name}'")]
    MissingBuiltinLib { name: String },

    /// Invalid target configuration.
    #[error(code = "ER014", message = "invalid target config: {target}: {message}")]
    InvalidTargetConfig {
        package: PackageId,
        target: TargetId,
        message: String,
    },
}
