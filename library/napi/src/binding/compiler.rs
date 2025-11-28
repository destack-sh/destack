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
