use napi_derive::napi;

/// The options for compiling.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct CompilerOptions {
    /// The options for evaluating.
    pub resolve: ResolveOptions,
}
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
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            default_int_width: 32,
            default_float_width: 32,
        }
    }
}
