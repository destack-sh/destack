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

// -- Compile Options (Top Level) --

/// The options for compiling a workspace.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompilerOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The number of worker threads to use.
    pub workers: u16,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: destack_compiler::default_workers(),
        }
    }
}

impl From<CompilerOptions> for destack_compiler::CompilerOptions {
    fn from(options: CompilerOptions) -> Self {
        Self {
            diagnostic: options.diagnostic.into(),
            workers: options.workers,
            ..Default::default()
        }
    }
}

/// Get the default compiler options.
#[napi(js_name = "defaultCompilerOptions")]
pub fn default_compile_options() -> CompilerOptions {
    CompilerOptions::default()
}
