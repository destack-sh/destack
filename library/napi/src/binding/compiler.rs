use destack_compiler as compiler;
use napi_derive::napi;

use super::{DiagnosticOptions, ResolveOptions};

/// How unresolved imports should be handled during resolve.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerResolveMode {
    /// Emit errors for unresolved imports.
    Strict,
    /// Emit warnings for unresolved imports and continue.
    Lenient,
}

impl From<CompilerResolveMode> for compiler::ResolveMode {
    fn from(mode: CompilerResolveMode) -> Self {
        match mode {
            CompilerResolveMode::Strict => compiler::ResolveMode::Strict,
            CompilerResolveMode::Lenient => compiler::ResolveMode::Lenient,
        }
    }
}

impl From<compiler::ResolveMode> for CompilerResolveMode {
    fn from(mode: compiler::ResolveMode) -> Self {
        match mode {
            compiler::ResolveMode::Strict => CompilerResolveMode::Strict,
            compiler::ResolveMode::Lenient => CompilerResolveMode::Lenient,
        }
    }
}

/// The options for compiling a workspace.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompilerOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The number of worker threads to use.
    pub workers: u16,
    /// Whether to follow imports automatically.
    pub follow_imports: bool,
    /// Options for resolving imports.
    pub import_resolve: ResolveOptions,
    /// How unresolved imports should be handled during resolve.
    pub resolve_mode: CompilerResolveMode,
    /// Whether to disallow ambiguous tree literal syntax.
    pub disallow_ambiguous_tree_literal: bool,
    /// Default integer width (if not specified).
    pub default_int_width: u16,
    /// Default float width (if not specified).
    pub default_float_width: u16,
    /// Whether to make prelude items available.
    pub inject_prelude: bool,
    /// Whether to load profile libraries by default.
    pub load_libs: bool,
    /// Whether to generate source maps.
    pub source_map: bool,
    /// Whether to use ternary expressions for simple if else value expressions.
    pub elaborate_with_ternary: bool,
    /// Whether to split multi declarator let statements into individual lets.
    pub elaborate_split_declarators: bool,
    /// Whether to make implicit returns explicit.
    pub elaborate_explicit_return: bool,
    /// Whether to wrap implicit casts inserted during elaborate in parentheses.
    pub elaborate_parenthesize_casts: bool,
    /// Whether to retain comptime expressions as comments after execution.
    pub retain_comptime_as_comment: bool,
    /// Maximum length of retained comptime comments.
    pub retain_comptime_comment_max_length: u32,
    /// Whether to overwrite existing files.
    pub emit_overwrite: bool,
    /// Whether to create parent directories if they do not exist.
    pub emit_create_dirs: bool,
    /// Dry run: report what would be written without writing.
    pub emit_dry_run: bool,
    /// Whether to collect detailed timing tags.
    pub timings: bool,
    /// Whether to validate builtin declaration libraries eagerly.
    pub validate_builtin_libs: bool,
    /// Whether to verify MIR after building it.
    pub verify_mir: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        compiler::CompilerOptions::default().into()
    }
}

impl From<CompilerOptions> for compiler::CompilerOptions {
    fn from(options: CompilerOptions) -> Self {
        // start from core defaults
        let mut core = compiler::CompilerOptions::default();

        // map resolve and type options
        core.diagnostic = options.diagnostic.into();
        core.workers = options.workers;
        core.follow_imports = options.follow_imports;
        core.import_resolve = options.import_resolve.into();
        core.resolve_mode = options.resolve_mode.into();
        core.disallow_ambiguous_tree_literal = options.disallow_ambiguous_tree_literal;
        core.default_int_width = options.default_int_width;
        core.default_float_width = options.default_float_width;
        core.inject_prelude = options.inject_prelude;
        core.load_libs = options.load_libs;
        core.source_map = options.source_map;

        // map elaborate options
        core.elaborate_with_ternary = options.elaborate_with_ternary;
        core.elaborate_split_declarators = options.elaborate_split_declarators;
        core.elaborate_explicit_return = options.elaborate_explicit_return;
        core.elaborate_parenthesize_casts = options.elaborate_parenthesize_casts;

        // map comptime retention
        core.retain_comptime_as_comment = options.retain_comptime_as_comment;
        core.retain_comptime_comment_max_length =
            usize::try_from(options.retain_comptime_comment_max_length).unwrap_or(usize::MAX);

        // map emit and validation options
        core.emit_overwrite = options.emit_overwrite;
        core.emit_create_dirs = options.emit_create_dirs;
        core.emit_dry_run = options.emit_dry_run;
        core.event_handler = None;
        core.timings = options.timings;
        core.validate_builtin_libs = options.validate_builtin_libs;
        core.verify_mir = options.verify_mir;
        core
    }
}

impl From<compiler::CompilerOptions> for CompilerOptions {
    fn from(options: compiler::CompilerOptions) -> Self {
        // map core compiler options into binding payload
        Self {
            diagnostic: DiagnosticOptions {
                error_warnings: options.diagnostic.error_warnings,
                suppress_errors: options.diagnostic.suppress_errors,
                suppress_warnings: options.diagnostic.suppress_warnings,
            },
            workers: options.workers,
            follow_imports: options.follow_imports,
            import_resolve: options.import_resolve.into(),
            resolve_mode: options.resolve_mode.into(),
            disallow_ambiguous_tree_literal: options.disallow_ambiguous_tree_literal,
            default_int_width: options.default_int_width,
            default_float_width: options.default_float_width,
            inject_prelude: options.inject_prelude,
            load_libs: options.load_libs,
            source_map: options.source_map,
            elaborate_with_ternary: options.elaborate_with_ternary,
            elaborate_split_declarators: options.elaborate_split_declarators,
            elaborate_explicit_return: options.elaborate_explicit_return,
            elaborate_parenthesize_casts: options.elaborate_parenthesize_casts,
            retain_comptime_as_comment: options.retain_comptime_as_comment,
            retain_comptime_comment_max_length: if options.retain_comptime_comment_max_length
                > u32::MAX as usize
            {
                u32::MAX
            } else {
                options.retain_comptime_comment_max_length as u32
            },
            emit_overwrite: options.emit_overwrite,
            emit_create_dirs: options.emit_create_dirs,
            emit_dry_run: options.emit_dry_run,
            timings: options.timings,
            validate_builtin_libs: options.validate_builtin_libs,
            verify_mir: options.verify_mir,
        }
    }
}

/// Get the default compiler options.
#[napi(js_name = "defaultCompilerOptions")]
pub fn default_compiler_options() -> CompilerOptions {
    // read defaults and normalize workers
    let mut options = CompilerOptions::default();
    if options.workers == 0 {
        options.workers = compiler::default_workers();
    }
    options
}
