use crate::{
    BuildRequirementError, BuildRequirementSet, DiagnosticAnchor, DiagnosticDefinition, TaskError,
};
use destack_builtin::LanguageSymbol;
use destack_compiler_macros::DefineError;
use destack_dir::{AnchoredGlobalNodeId, GlobalScopeId, GlobalSymbolId, StaticKey, StringId};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{Program, TargetId};

/// Errors during the resolve phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Resolve)]
pub enum ResolveError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for build requirement.
    #[error(code = "ER000", r#yield)]
    Yield { requirement: BuildRequirementSet },

    /// Yield requirement has failed.
    #[error(code = "ER001", yield_failed)]
    UnsatisfiedRequirement { requirement: BuildRequirementSet },

    /// Task was skipped due to stale versions.
    #[error(code = "ER002", message = "task skipped")]
    Skipped,

    // -------------------------------------------------------------------------
    // 1xx: Symbol lookup
    // -------------------------------------------------------------------------
    /// Use of undeclared symbol.
    #[error(code = "ER100", message = "missing symbol {key}")]
    UndeclaredSymbol {
        node: AnchoredGlobalNodeId,
        scope: GlobalScopeId,
        key: StaticKey,
    },

    /// Use of missing symbol.
    #[error(code = "ER101", message = "missing symbol {key}")]
    MissingSymbol {
        node: AnchoredGlobalNodeId,
        scope: GlobalScopeId,
        via_module: Option<ModuleId>,
        key: StaticKey,
    },

    /// Use of ambiguous symbol.
    #[error(code = "ER102", message = "ambiguous symbol {key}")]
    AmbiguousSymbol {
        node: AnchoredGlobalNodeId,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        key: StaticKey,
    },

    /// Cyclic symbol reference (re-export chain forms a cycle).
    #[error(code = "ER103", message = "cyclic reference (recursive) to '{symbol}'")]
    CyclicSymbol {
        node: AnchoredGlobalNodeId,
        symbol: GlobalSymbolId,
    },

    /// Export clause references a local binding that is not declared.
    #[error(code = "ER104", message = "missing exported local binding '{name}'")]
    MissingExportBinding {
        node: AnchoredGlobalNodeId,
        name: StringId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Module / target resolution
    // -------------------------------------------------------------------------
    /// Unresolved module.
    #[error(code = "ER200", message = "unresolved module '{target}'")]
    UnresolvedModule {
        node: AnchoredGlobalNodeId,
        target: StringId,
    },

    /// Missing target for a control flow expression.
    #[error(code = "ER201", message = "missing target")]
    MissingTarget {
        node: AnchoredGlobalNodeId,
        target: Option<StringId>,
    },

    /// Invalid target for a control flow expression.
    #[error(code = "ER202", message = "invalid target")]
    InvalidTarget {
        node: AnchoredGlobalNodeId,
        target: Option<StringId>,
        target_node: AnchoredGlobalNodeId,
    },

    /// Invalid loader type in import attributes.
    #[error(code = "ER203", message = "invalid import attribute type '{value}'")]
    InvalidImportAttributeType {
        node: AnchoredGlobalNodeId,
        value: String,
    },

    /// Unknown protocol scheme in an import specifier.
    #[error(code = "ER204", message = "unknown protocol scheme '{scheme}'")]
    UnknownProtocolScheme {
        node: AnchoredGlobalNodeId,
        scheme: StringId,
    },

    /// Builtin module is not available for the current runtime.
    #[error(
        code = "ER205",
        message = "builtin module '{target}' is not supported for runtime '{runtime}'"
    )]
    UnsupportedBuiltinModule {
        node: AnchoredGlobalNodeId,
        target: StringId,
        runtime: String,
    },

    /// Unknown builtin module in a recognized protocol namespace.
    #[error(code = "ER206", message = "no such built-in module: {target}")]
    UnknownBuiltinModule {
        node: AnchoredGlobalNodeId,
        target: StringId,
    },

    /// Bare builtin module import must use an explicit protocol prefix.
    #[error(
        code = "ER207",
        message = "builtin module '{target}' must use the '{suggested}' protocol form"
    )]
    UnprefixedBuiltinModule {
        node: AnchoredGlobalNodeId,
        target: StringId,
        suggested: StringId,
    },

    /// Internal protocol import is disabled by compiler policy.
    #[error(
        code = "ER208",
        message = "internal module import '{target}' is disabled by compiler policy"
    )]
    UnsupportedInternalModule {
        node: AnchoredGlobalNodeId,
        target: StringId,
    },
    // -------------------------------------------------------------------------
    // 3xx: Dependencies / cycles
    // -------------------------------------------------------------------------
    /// Circular dependency.
    #[error(code = "ER300", message = "circular dependency")]
    CircularDependency {
        node: AnchoredGlobalNodeId,
        depends_on: Vec<AnchoredGlobalNodeId>,
    },

    // -------------------------------------------------------------------------
    // 4xx: Builtins / config
    // -------------------------------------------------------------------------
    /// Missing language item (builtin not found).
    #[error(code = "ER400", message = "missing language item '{item}'")]
    MissingLanguageSymbol { item: LanguageSymbol },

    /// Missing builtin library.
    #[error(code = "ER401", message = "missing builtin lib '{name}'")]
    MissingBuiltinLib { name: String },

    /// Conflicting builtin lib versions.
    #[error(
        code = "ER402",
        message = "conflicting builtin lib versions for '{base}': {libs}"
    )]
    ConflictingBuiltinLibVersions { base: String, libs: String },

    /// Invalid target configuration.
    #[error(code = "ER403", message = "invalid target config: {target}: {message}")]
    InvalidTargetConfig {
        package: PackageId,
        target: TargetId,
        message: String,
    },

    /// Invalid intrinsic binding in builtin modules.
    #[error(code = "ER404", message = "invalid intrinsic binding: {message}")]
    InvalidIntrinsicBinding {
        node: AnchoredGlobalNodeId,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[error(code = "ER900", message = "unsupported {node}")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },

    /// Invalid static if decorator.
    #[error(code = "ER901", message = "invalid static if: {message}")]
    InvalidStaticIf {
        node: AnchoredGlobalNodeId,
        message: String,
    },
}
