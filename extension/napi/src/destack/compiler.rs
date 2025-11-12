use napi_derive::napi;
use dyst_compiler;

/// The options for compiling.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct CompilerOptions {
    /// Default integer width.
    pub default_int_width: u16,
    /// Default float width.
    pub default_float_width: u16,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            default_int_width: 32,
            default_float_width: 32,
        }
    }
}

impl From<CompilerOptions> for dyst_compiler::CompilerOptions {
    fn from(options: CompilerOptions) -> Self {
        Self {
            default_int_width: options.default_int_width,
            default_float_width: options.default_float_width,
        }
    }
}

/// Get the default compiler options.
#[napi(js_name = "defaultCompilerOptions")]
pub fn default_compiler_options() -> CompilerOptions {
    CompilerOptions::default()
}

