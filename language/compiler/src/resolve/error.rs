use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_dir::{GlobalScopeId, GlobalSymbolId, LanguageItem};
use destack_source::{ModuleId, PackageId, TargetId};

/// Errors during the resolve phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Resolve)]
pub enum ResolveError {
    // -------------------------------------------------------------------------
    // 1xx: Symbol lookup
    // -------------------------------------------------------------------------
    /// Use of undeclared symbol.
    #[diagnostic(code = "ER100", message = "missing symbol {key}")]
    UndeclaredSymbol {
        anchor: DiagnosticAnchor,
        scope: GlobalScopeId,
        key: String,
    },

    /// Use of missing symbol.
    #[diagnostic(code = "ER101", message = "missing symbol {key}")]
    MissingSymbol {
        anchor: DiagnosticAnchor,
        scope: GlobalScopeId,
        via_module: Option<ModuleId>,
        key: String,
    },

    /// Use of ambiguous symbol.
    #[diagnostic(code = "ER102", message = "ambiguous symbol {key}")]
    AmbiguousSymbol {
        anchor: DiagnosticAnchor,
        scope: GlobalScopeId,
        symbol: GlobalSymbolId,
        key: String,
    },

    /// Cyclic symbol reference (re-export chain forms a cycle).
    #[diagnostic(code = "ER103", message = "cyclic reference")]
    CyclicSymbol {
        anchor: DiagnosticAnchor,
        symbol: GlobalSymbolId,
    },

    /// Export clause references a local binding that is not declared.
    #[diagnostic(code = "ER104", message = "missing exported local binding '{name}'")]
    MissingExportBinding {
        anchor: DiagnosticAnchor,
        name: String,
    },

    // -------------------------------------------------------------------------
    // 2xx: Module / target resolution
    // -------------------------------------------------------------------------
    /// Unresolved module.
    #[diagnostic(code = "ER200", message = "unresolved module '{target}'")]
    UnresolvedModule {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Missing target for a control flow expression.
    #[diagnostic(code = "ER201", message = "missing target")]
    MissingTarget { anchor: DiagnosticAnchor },

    /// Invalid target for a control flow expression.
    #[diagnostic(code = "ER202", message = "invalid target")]
    InvalidTarget { anchor: DiagnosticAnchor },

    /// Invalid loader type in import attributes.
    #[diagnostic(code = "ER203", message = "invalid import attribute type '{value}'")]
    InvalidImportAttributeType {
        anchor: DiagnosticAnchor,
        value: String,
    },

    /// Unknown protocol scheme in an import specifier.
    #[diagnostic(code = "ER204", message = "unknown protocol scheme '{scheme}'")]
    UnknownProtocolScheme {
        anchor: DiagnosticAnchor,
        scheme: String,
    },

    /// Builtin module is not available for the current runtime.
    #[diagnostic(
        code = "ER205",
        message = "library module '{target}' is not supported for runtime '{runtime}'"
    )]
    UnsupportedBuiltinModule {
        anchor: DiagnosticAnchor,
        target: String,
        runtime: String,
    },

    /// Unknown library module in a recognized protocol namespace.
    #[diagnostic(code = "ER206", message = "no such built-in module: {target}")]
    UnknownBuiltinModule {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Bare library module import must use an explicit protocol prefix.
    #[diagnostic(
        code = "ER207",
        message = "library module '{target}' must use the '{suggested}' protocol form"
    )]
    UnprefixedBuiltinModule {
        anchor: DiagnosticAnchor,
        target: String,
        suggested: String,
    },

    /// Internal protocol import is disabled by compiler policy.
    #[diagnostic(
        code = "ER208",
        message = "internal module import '{target}' is disabled by compiler policy"
    )]
    UnsupportedInternalModule {
        anchor: DiagnosticAnchor,
        target: String,
    },
    // -------------------------------------------------------------------------
    // 3xx: Dependencies / cycles
    // -------------------------------------------------------------------------
    /// Circular dependency.
    #[diagnostic(code = "ER300", message = "circular dependency")]
    CircularDependency {
        anchor: DiagnosticAnchor,
        depends_on: Vec<DiagnosticAnchor>,
    },

    // -------------------------------------------------------------------------
    // 4xx: Language library / config
    // -------------------------------------------------------------------------
    /// Missing language item.
    #[diagnostic(code = "ER400", message = "missing language item '{item}'")]
    MissingLanguageItem {
        anchor: DiagnosticAnchor,
        item: LanguageItem,
    },

    /// Missing library package.
    #[diagnostic(code = "ER401", message = "missing library package '{name}'")]
    MissingLibraryPackage {
        anchor: DiagnosticAnchor,
        name: String,
    },

    /// Conflicting library package versions.
    #[diagnostic(
        code = "ER402",
        message = "conflicting library package versions for '{base}': {libs}"
    )]
    ConflictingLibraryPackageVersions {
        anchor: DiagnosticAnchor,
        base: String,
        libs: String,
    },

    /// Invalid target configuration.
    #[diagnostic(code = "ER403", message = "invalid target config: {target}: {message}")]
    InvalidTargetConfig {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        message: String,
    },

    /// Invalid intrinsic binding in library modules.
    #[diagnostic(code = "ER404", message = "invalid intrinsic binding: {message}")]
    InvalidIntrinsicBinding {
        anchor: DiagnosticAnchor,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[diagnostic(code = "ER900", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },

    /// Invalid static if decorator.
    #[diagnostic(code = "ER901", message = "invalid static if: {message}")]
    InvalidStaticIf {
        anchor: DiagnosticAnchor,
        message: String,
    },

    /// Internal resolve failure.
    #[diagnostic(code = "ER902", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
