use dyst_dir::Program;
use dyst_source::{
    DiagnosticCollection, DiagnosticOptions, PrintOptions,
    print_diagnostics as print_diagnostics_impl,
};

use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct DiagnosticArgs {
    /// Error on the given warning codes (like W001).
    #[arg(long, value_delimiter = ',', value_name = "CODES")]
    pub error_warnings: Vec<String>,

    /// Suppress the given error codes (like E001) as warnings.
    #[arg(long, value_delimiter = ',', value_name = "CODES")]
    pub suppress_errors: Vec<String>,

    /// Suppress the given warning codes (like W001).
    #[arg(long, value_delimiter = ',', value_name = "CODES")]
    pub suppress_warnings: Vec<String>,
}

impl From<DiagnosticArgs> for DiagnosticOptions {
    fn from(args: DiagnosticArgs) -> Self {
        DiagnosticOptions {
            error_warnings: args.error_warnings,
            suppress_errors: args.suppress_errors,
            suppress_warnings: args.suppress_warnings,
        }
    }
}

/// Print diagnostics (and suggestions) to the console.
pub(crate) fn print_diagnostics(program: &Program, diagnostics: &DiagnosticCollection) {
    let options = PrintOptions::new()
        .with_line_width(program.language.formatting.line_width as u32)
        .with_module_count(program.modules.len());
    print_diagnostics_impl(&program.files, diagnostics, options);
}
