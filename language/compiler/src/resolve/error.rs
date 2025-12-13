use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::define_errors;
use destack_dir::{GlobalNodeIdAny, GlobalScopeId, GlobalSymbolId, StaticKey, StringId};
use destack_source::ModuleId;
use destack_workspace::Program;

define_errors!(Resolve, {
    /// Wait for task dependency.
    #[error_yield]
    "ER000" = Yield {
        dependency: TaskDependency,
    } => "pending dependency",

    /// Yield dependency has failed.
    #[error_yield_failed]
    "ER001" = UnsatisfiedDependency {
        dependency: TaskDependency,
    } => "unsatisfied dependency",

    /// Unsupported node.
    "ER002" = UnsupportedConstruct {
        node: GlobalNodeIdAny,
    } => {
        format!("unsupported {}", node.local_id.ty.name())
    },

    /// Circular dependency.
    "ER003" = CircularDependency {
        node: GlobalNodeIdAny,
        depends_on: Vec<GlobalNodeIdAny>,
    } => "circular dependency",

    /// Use of undeclared symbol.
    "ER004" = UndeclaredSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        key: StaticKey,
    } => {
        let key = key.debug_string(&program.strings);
        format!("missing symbol {key}")
    },

    /// Use of missing symbol.
    "ER005" = MissingSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        via_module: Option<ModuleId>,
        key: StaticKey,
    } => {
        let key = key.debug_string(&program.strings);
        if let &Some(via_module) = via_module {
            let via_module = program.modules.get(via_module).read().uri.to_string();
            format!("missing symbol {key} in '{via_module}'")
        } else {
            format!("missing symbol {key}")
        }
    },

    /// Use of ambiguous symbol.
    "ER006" = AmbiguousSymbol {
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        key: StaticKey,
    } => {
        let key = key.debug_string(&program.strings);
        format!("ambiguous symbol {key}")
    },

    /// Unresolved module.
    "ER007" = UnresolvedModule {
        node: GlobalNodeIdAny,
        target: StringId,
    } => {
        let target = program.strings.get(*target).to_string();
        format!("unresolved module '{target}'")
    },

    /// Cyclic symbol reference (re-export chain forms a cycle).
    "ER008" = CyclicSymbol {
        node: GlobalNodeIdAny,
        symbol: GlobalSymbolId,
    } => "cyclic symbol reference",

    /// Self type used outside of a type context.
    "ER009" = MissingSelf {
        node: GlobalNodeIdAny,
    } => "`Self` type can only be used inside a class, struct, or enum",

    /// Missing target for a control flow expression.
    "ER010" = MissingTarget {
        node: GlobalNodeIdAny,
        target: Option<StringId>,
    } => {
        let target = target
            .map(|t| program.strings.get(t).to_string())
            .unwrap_or_default();
        format!("missing target {target}")
    },

    /// Invalid target for a control flow expression.
    "ER011" = InvalidTarget {
        node: GlobalNodeIdAny,
        target: Option<StringId>,
        target_node: GlobalNodeIdAny,
    } => {
        let target = target
            .map(|t| program.strings.get(t).to_string())
            .unwrap_or_default();
        format!("invalid target {target}")
    },
});
