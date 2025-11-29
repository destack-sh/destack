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

// -- Resolve Options (Compiler) --

/// The options for type resolution.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct TypeResolveOptions {
    /// Default integer width (if not specified).
    pub default_int_width: u16,
    /// Default float width (if not specified).
    pub default_float_width: u16,
}

impl Default for TypeResolveOptions {
    fn default() -> Self {
        Self {
            default_int_width: 32,
            default_float_width: 32,
        }
    }
}

impl From<TypeResolveOptions> for destack_compiler::ResolveOptions {
    fn from(options: TypeResolveOptions) -> Self {
        Self {
            default_int_width: options.default_int_width,
            default_float_width: options.default_float_width,
        }
    }
}

// -- Validate Options --

/// The options for validating.
#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidateOptions {}

impl From<ValidateOptions> for destack_compiler::ValidateOptions {
    fn from(_options: ValidateOptions) -> Self {
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

// -- Execute Options --

/// The options for executing static expressions.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct ExecuteOptions {
    /// Whether to fold constants.
    pub fold_constants: bool,
    /// Whether to execute static expressions.
    pub execute_static: bool,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            fold_constants: true,
            execute_static: true,
        }
    }
}

impl From<ExecuteOptions> for destack_compiler::ExecuteOptions {
    fn from(options: ExecuteOptions) -> Self {
        Self {
            fold_constants: options.fold_constants,
            execute_static: options.execute_static,
        }
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

// -- Build Options --

/// The options for building.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct BuildOptions {
    /// Whether to generate source maps.
    pub source_map: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self { source_map: true }
    }
}

impl From<BuildOptions> for destack_compiler::BuildOptions {
    fn from(options: BuildOptions) -> Self {
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
    /// The options for type resolution.
    pub resolve: TypeResolveOptions,
    /// The options for validating.
    pub validate: ValidateOptions,
    /// The options for lowering.
    pub lower: LowerOptions,
    /// The options for executing.
    pub execute: ExecuteOptions,
    /// The options for optimizing.
    pub optimize: OptimizeOptions,
    /// The options for building.
    pub build: BuildOptions,
    /// The options for linking.
    pub link: LinkOptions,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: destack_compiler::default_workers(),
            import: ImportOptions::default(),
            resolve: TypeResolveOptions::default(),
            validate: ValidateOptions::default(),
            lower: LowerOptions::default(),
            execute: ExecuteOptions::default(),
            optimize: OptimizeOptions::default(),
            build: BuildOptions::default(),
            link: LinkOptions::default(),
        }
    }
}

impl From<CompileOptions> for destack_compiler::CompileOptions {
    fn from(options: CompileOptions) -> Self {
        Self {
            diagnostic: options.diagnostic.into(),
            workers: options.workers,
            import: options.import.into(),
            resolve: options.resolve.into(),
            validate: options.validate.into(),
            lower: options.lower.into(),
            execute: options.execute.into(),
            optimize: options.optimize.into(),
            build: options.build.into(),
            link: options.link.into(),
        }
    }
}

/// Get the default compiler options.
#[napi(js_name = "defaultCompileOptions")]
pub fn default_compile_options() -> CompileOptions {
    CompileOptions::default()
}
