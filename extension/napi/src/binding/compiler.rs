use napi_derive::napi;

/// The options for compiling.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct CompilerOptions {
    /// The options for evaluating.
    pub resolve: ResolveOptions,
}

#[allow(clippy::derivable_impls)]
impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            resolve: ResolveOptions::default(),
        }
    }
}

impl From<CompilerOptions> for dyst_compiler::CompilerOptions {
    fn from(options: CompilerOptions) -> Self {
        Self {
            import: dyst_compiler::ImportOptions::default(),
            resolve: options.resolve.into(),
            validate: dyst_compiler::ValidateOptions::default(),
            execute: dyst_compiler::ExecuteOptions::default(),
            optimize: dyst_compiler::OptimizeOptions::default(),
            build: dyst_compiler::BuildOptions::default(),
        }
    }
}

/// Get the default compiler options.
#[napi(js_name = "defaultCompilerOptions")]
pub fn default_compiler_options() -> CompilerOptions {
    CompilerOptions::default()
}

#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct ResolveOptions {
    /// Default integer width.
    pub default_int_width: u16,
    /// Default float width.
    pub default_float_width: u16,
    /// Whether to implicitly type unannotated expressions as any.
    pub implicit_any_type: bool,
    /// Whether to resolve overimported operators.
    pub overimport_operators: bool,
    /// Whether to resolve overimported functions.
    pub overimport_functions: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            default_int_width: 32,
            default_float_width: 32,
            implicit_any_type: false,
            overimport_operators: true,
            overimport_functions: true,
        }
    }
}

impl From<ResolveOptions> for dyst_compiler::ResolveOptions {
    fn from(options: ResolveOptions) -> Self {
        Self {
            default_int_width: options.default_int_width,
            default_float_width: options.default_float_width,
            implicit_any_type: options.implicit_any_type,
            overimport_operators: options.overimport_operators,
            overimport_functions: options.overimport_functions,
        }
    }
}
