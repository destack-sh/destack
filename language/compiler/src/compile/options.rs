use super::parallel::default_workers as resolve_default_workers;
#[cfg(test)]
use crate::tests::scenario::CompilerScenarioEventHandler;
use destack_source::DiagnosticOptions;

/// Get the default number of worker threads.
pub fn default_workers() -> u16 {
    resolve_default_workers()
}

/// How unresolved imports should be handled during resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveMode {
    /// Emit errors for unresolved imports.
    Strict,
    /// Emit warnings for unresolved imports and continue.
    Lenient,
}

/// Language compatibility options for one module.
#[derive(Debug, Clone, Copy)]
pub struct ModuleCheckOptions {
    /// Allow TypeScript modules.
    pub allow_ts: bool,
    /// Type check TypeScript modules.
    pub check_ts: bool,
    /// Allow JavaScript modules.
    pub allow_js: bool,
    /// Type check JavaScript modules.
    pub check_js: bool,
    /// Skip declaration module checks.
    pub skip_lib_check: bool,
}

impl Default for ModuleCheckOptions {
    fn default() -> Self {
        Self::from_workspace(&destack_workspace::CompilerOptions::default())
    }
}

impl ModuleCheckOptions {
    /// Build module compatibility options from workspace compiler options.
    pub fn from_workspace(options: &destack_workspace::CompilerOptions) -> Self {
        Self {
            allow_ts: options.allow_ts,
            check_ts: options.check_ts,
            allow_js: options.allow_js,
            check_js: options.check_js,
            skip_lib_check: options.skip_lib_check,
        }
    }

    /// Apply TypeScript compiler option overrides.
    pub fn apply_typescript(&mut self, options: &destack_workspace::TsCompilerOptions) {
        self.allow_js = options.allow_js;
        self.check_js = options.check_js;
        self.skip_lib_check = options.skip_lib_check;
    }
}

/// The options for compiling a Workspace.
#[derive(Clone)]
pub struct CompilerOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The number of worker threads to use.
    pub workers: u16,

    /// Whether to follow imports automatically.
    pub follow_imports: bool,
    /// Options for resolving imports.
    pub import_resolve: destack_resolver::ResolveOptions,
    /// How unresolved imports should be handled during resolve.
    pub resolve_mode: ResolveMode,
    /// Whether to disallow ambiguous tree literal syntax.
    pub disallow_ambiguous_tree_literal: bool,

    /// Default integer width (if not specified).
    pub default_int_width: u16,
    /// Default float width (if not specified).
    pub default_float_width: u16,
    /// Whether to make prelude items (Add, Type, deprecated, etc.) available.
    /// When true, prelude items resolve without explicit imports.
    pub inject_prelude: bool,
    /// Whether to load profile libraries (es*, dom, std, etc.) by default.
    pub load_libraries: bool,

    /// Whether to generate source maps.
    pub source_map: bool,

    /// Whether to use ternary expressions for simple if-else value expressions.
    pub elaborate_with_ternary: bool,
    /// Whether to split multi-declarator let statements into individual lets.
    /// e.g., `let a = 1, b = 2` → `let a = 1; let b = 2;`
    pub elaborate_split_declarators: bool,
    /// Whether to make implicit returns explicit.
    /// e.g., `function f() { 42 }` → `function f() { return 42; }`
    pub elaborate_explicit_return: bool,
    /// Whether to wrap implicit casts inserted during elaborate in parentheses.
    pub elaborate_parenthesize_casts: bool,

    /// Whether to retain comptime expressions as comments after execution.
    pub retain_comptime_as_comment: bool,
    /// Maximum length of retained comptime comments (after compaction).
    /// A value of 0 disables comment retention entirely.
    pub retain_comptime_comment_max_length: usize,

    /// Whether to overwrite existing files.
    pub emit_overwrite: bool,
    /// Whether to create parent directories if they don't exist.
    pub emit_create_dirs: bool,
    /// Dry run: report what would be written without actually writing.
    pub emit_dry_run: bool,

    /// Optional internal scenario event handler for deterministic interleaving tests.
    #[cfg(test)]
    pub(crate) scenario_event_handler: Option<CompilerScenarioEventHandler>,

    /// Whether to collect detailed timing tags.
    pub timings: bool,
    /// Whether to validate builtin declaration libs eagerly.
    pub validate_builtin_libs: bool,
    /// Whether to verify MIR after building it (internal debug/test builds).
    pub verify_mir: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: resolve_default_workers(),

            follow_imports: true,
            import_resolve: destack_resolver::ResolveOptions::default(),
            resolve_mode: ResolveMode::Strict,
            disallow_ambiguous_tree_literal: false,

            default_int_width: 32,
            default_float_width: 64,
            inject_prelude: true,
            load_libraries: true,

            source_map: true,

            elaborate_with_ternary: true,
            elaborate_split_declarators: true,
            elaborate_explicit_return: true,
            elaborate_parenthesize_casts: false,
            retain_comptime_as_comment: false,
            retain_comptime_comment_max_length: 120,

            emit_overwrite: true,
            emit_create_dirs: true,
            emit_dry_run: false,
            #[cfg(test)]
            scenario_event_handler: None,
            timings: false,
            validate_builtin_libs: false,
            verify_mir: true,
        }
    }
}

impl std::fmt::Debug for CompilerOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("CompilerOptions");
        debug
            .field("diagnostic", &self.diagnostic)
            .field("workers", &self.workers)
            .field("follow_imports", &self.follow_imports)
            .field("import_resolve", &self.import_resolve)
            .field("resolve_mode", &self.resolve_mode)
            .field(
                "disallow_ambiguous_tree_literal",
                &self.disallow_ambiguous_tree_literal,
            )
            .field("default_int_width", &self.default_int_width)
            .field("default_float_width", &self.default_float_width)
            .field("inject_prelude", &self.inject_prelude)
            .field("load_libraries", &self.load_libraries)
            .field("source_map", &self.source_map)
            .field("elaborate_with_ternary", &self.elaborate_with_ternary)
            .field(
                "elaborate_split_declarators",
                &self.elaborate_split_declarators,
            )
            .field("elaborate_explicit_return", &self.elaborate_explicit_return)
            .field(
                "elaborate_parenthesize_casts",
                &self.elaborate_parenthesize_casts,
            )
            .field(
                "retain_comptime_as_comment",
                &self.retain_comptime_as_comment,
            )
            .field(
                "retain_comptime_comment_max_length",
                &self.retain_comptime_comment_max_length,
            )
            .field("emit_overwrite", &self.emit_overwrite)
            .field("emit_create_dirs", &self.emit_create_dirs)
            .field("emit_dry_run", &self.emit_dry_run);

        #[cfg(test)]
        debug.field(
            "scenario_event_handler",
            &self.scenario_event_handler.is_some(),
        );

        debug
            .field("timings", &self.timings)
            .field("validate_builtin_libs", &self.validate_builtin_libs)
            .field("verify_mir", &self.verify_mir)
            .finish()
    }
}
