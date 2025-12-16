use napi_derive::napi;

// -- Diagnostic Options --

/// Diagnostic options for re-mapping errors and warnings.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct DiagnosticOptions {
    /// Which warning codes to error on (as errors).
    pub error_warnings: Vec<String>,
    /// Which error codes to suppress (as warnings).
    pub suppress_errors: Vec<String>,
    /// Which warning codes to suppress.
    pub suppress_warnings: Vec<String>,
}

impl From<DiagnosticOptions> for destack_source::DiagnosticOptions {
    fn from(options: DiagnosticOptions) -> Self {
        Self {
            error_warnings: options.error_warnings,
            suppress_errors: options.suppress_errors,
            suppress_warnings: options.suppress_warnings,
        }
    }
}

// -- Import Options --

/// The options for importing modules.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Whether to follow imports automatically.
    pub follow_imports: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            follow_imports: true,
        }
    }
}

impl From<ImportOptions> for destack_compiler::ImportOptions {
    fn from(options: ImportOptions) -> Self {
        Self {
            follow_imports: options.follow_imports,
            resolve: destack_resolver::ResolveOptions::default(),
        }
    }
}

/// Options for binding.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct BindOptions {}

impl From<BindOptions> for destack_compiler::BindOptions {
    fn from(_options: BindOptions) -> Self {
        Self {}
    }
}

// -- Resolve Options (Compiler) --

/// The options for type resolution.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct TypeResolveOptions {
    /// Default integer width (if not specified).
    pub default_int_width: u16,
    /// Default float width (if not specified).
    pub default_float_width: u16,
    /// Whether to make prelude items (Add, Type, deprecated) available without imports.
    pub inject_prelude: Option<bool>,
}

impl Default for TypeResolveOptions {
    fn default() -> Self {
        Self {
            default_int_width: 32,
            default_float_width: 32,
            inject_prelude: None,
        }
    }
}

impl From<TypeResolveOptions> for destack_compiler::ResolveOptions {
    fn from(options: TypeResolveOptions) -> Self {
        Self {
            default_int_width: options.default_int_width,
            default_float_width: options.default_float_width,
            inject_prelude: options.inject_prelude.unwrap_or(true),
        }
    }
}

// -- Analyze Options --

/// The options for analyzing.
#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalyzeOptions {}

impl From<AnalyzeOptions> for destack_compiler::AnalyzeOptions {
    fn from(_options: AnalyzeOptions) -> Self {
        Self {}
    }
}

// -- Lower Options --

/// The options for lowering.
#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LowerOptions {}

impl From<LowerOptions> for destack_compiler::LowerOptions {
    fn from(_options: LowerOptions) -> Self {
        Self {}
    }
}

// -- Optimize Options --

/// The options for optimizing.
#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OptimizeOptions {}

impl From<OptimizeOptions> for destack_compiler::OptimizeOptions {
    fn from(_options: OptimizeOptions) -> Self {
        Self {}
    }
}

// -- Generate Options --

/// The options for code generation.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct GenerateOptions {
    /// Whether to generate source maps.
    pub source_map: bool,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self { source_map: true }
    }
}

impl From<GenerateOptions> for destack_compiler::GenerateOptions {
    fn from(options: GenerateOptions) -> Self {
        Self {
            source_map: options.source_map,
        }
    }
}

// -- Link Options --

/// The options for linking.
#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkOptions {}

impl From<LinkOptions> for destack_compiler::LinkOptions {
    fn from(_options: LinkOptions) -> Self {
        Self {}
    }
}

// -- Emit Options --

/// The options for emitting compiled output.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct EmitOptions {
    /// Whether to overwrite existing files.
    pub overwrite: bool,
    /// Whether to create parent directories if they don't exist.
    pub create_dirs: bool,
    /// Dry run: report what would be written without actually writing.
    pub dry_run: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            overwrite: true,
            create_dirs: true,
            dry_run: false,
        }
    }
}

impl From<EmitOptions> for destack_compiler::EmitOptions {
    fn from(options: EmitOptions) -> Self {
        Self {
            overwrite: options.overwrite,
            create_dirs: options.create_dirs,
            dry_run: options.dry_run,
        }
    }
}

// -- Compile Options (Top Level) --

/// The options for compiling a workspace.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The number of worker threads to use.
    pub workers: u16,
    /// The options for importing.
    pub import: ImportOptions,
    /// The options for binding.
    pub bind: BindOptions,
    /// The options for type resolution.
    pub resolve: TypeResolveOptions,
    /// The options for analyzing.
    pub analyze: AnalyzeOptions,
    /// The options for lowering.
    pub lower: LowerOptions,
    /// The options for optimizing.
    pub optimize: OptimizeOptions,
    /// The options for code generation.
    pub generate: GenerateOptions,
    /// The options for linking.
    pub link: LinkOptions,
    /// The options for emitting.
    pub emit: EmitOptions,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: destack_compiler::default_workers(),
            import: ImportOptions::default(),
            bind: BindOptions::default(),
            resolve: TypeResolveOptions::default(),
            analyze: AnalyzeOptions::default(),
            lower: LowerOptions::default(),
            optimize: OptimizeOptions::default(),
            generate: GenerateOptions::default(),
            link: LinkOptions::default(),
            emit: EmitOptions::default(),
        }
    }
}

impl From<CompileOptions> for destack_compiler::CompileOptions {
    fn from(options: CompileOptions) -> Self {
        Self {
            diagnostic: options.diagnostic.into(),
            workers: options.workers,
            import: options.import.into(),
            bind: options.bind.into(),
            resolve: options.resolve.into(),
            analyze: options.analyze.into(),
            lower: options.lower.into(),
            optimize: options.optimize.into(),
            generate: options.generate.into(),
            link: options.link.into(),
            emit: options.emit.into(),
        }
    }
}

/// Get the default compiler options.
#[napi(js_name = "defaultCompileOptions")]
pub fn default_compile_options() -> CompileOptions {
    CompileOptions::default()
}
