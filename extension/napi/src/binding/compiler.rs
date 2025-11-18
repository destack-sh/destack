use napi_derive::napi;

/// The options for compiling.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct CompileOptions {
    /// The options for evaluating.
    pub resolve: ResolveOptions,
}

#[allow(clippy::derivable_impls)]
impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            resolve: ResolveOptions::default(),
        }
    }
}

impl From<CompileOptions> for dyst_compiler::CompileOptions {
    fn from(options: CompileOptions) -> Self {
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
#[napi(js_name = "defaultCompileOptions")]
pub fn default_compiler_options() -> CompileOptions {
    CompileOptions::default()
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
    /// Whether to resolve overloaded operators.
    pub overload_operators: bool,
    /// Whether to resolve overloaded functions.
    pub overload_functions: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            default_int_width: 32,
            default_float_width: 32,
            implicit_any_type: false,
            overload_operators: true,
            overload_functions: true,
        }
    }
}

impl From<ResolveOptions> for dyst_compiler::ResolveOptions {
    fn from(options: ResolveOptions) -> Self {
        Self {
            default_int_width: options.default_int_width,
            default_float_width: options.default_float_width,
            implicit_any_type: options.implicit_any_type,
            overload_operators: options.overload_operators,
            overload_functions: options.overload_functions,
        }
    }
}
