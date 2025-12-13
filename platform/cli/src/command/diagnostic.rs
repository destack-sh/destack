use destack_compiler::DiagnosticRegistry;
use destack_parser::source_colorizer;
use destack_source::{
    DiagnosticCollection, DiagnosticOptions, PrintOptions,
    print_diagnostics as print_diagnostics_impl,
};
use destack_workspace::Program;

use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct DiagnosticArgs {
    /// Error on the given warning codes (like WR001).
    #[arg(long, value_delimiter = ',', value_name = "CODES", value_parser = validate_warning_code)]
    pub error_warnings: Vec<String>,

    /// Suppress the given error codes (like ER004) as warnings.
    #[arg(long, value_delimiter = ',', value_name = "CODES", value_parser = validate_error_code)]
    pub suppress_errors: Vec<String>,

    /// Suppress the given warning codes (like WR001).
    #[arg(long, value_delimiter = ',', value_name = "CODES", value_parser = validate_warning_code)]
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
        .with_module_count(program.modules.len())
        .with_colorizer(source_colorizer());
    print_diagnostics_impl(&program.files, diagnostics, options);
}

/// Validate that a warning code is known.
fn validate_warning_code(code: &str) -> Result<String, String> {
    if DiagnosticRegistry::is_valid_warning_code(code) {
        Ok(code.to_string())
    } else {
        Err(format!("unknown warning code: {code}"))
    }
}

/// Validate that an error code is known.
fn validate_error_code(code: &str) -> Result<String, String> {
    if DiagnosticRegistry::is_valid_error_code(code) {
        Ok(code.to_string())
    } else {
        Err(format!("unknown error code: {code}"))
    }
}
