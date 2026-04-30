use destack_compiler::DiagnosticRegistry;
use destack_parser::source_colorizer;
use destack_source::{
    DiagnosticCollection, DiagnosticOptions, PrintOptions,
    print_diagnostics as print_diagnostics_impl,
};
use destack_workspace::{Repository, Revision};

use clap::Args;

use crate::error::{CliError, CliResult};

#[derive(Args, Debug, Clone, Default)]
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
pub fn print_diagnostics(
    repository: &Repository,
    revision: Revision,
    diagnostics: &DiagnosticCollection,
) {
    let module_count = repository
        .module_ids(revision)
        .map(|modules| modules.len())
        .unwrap_or(0);
    let options = PrintOptions::new()
        .with_line_width(100)
        .with_module_count(module_count)
        .with_colorizer(source_colorizer());
    let file_for_id = |file_id| {
        repository
            .file(revision, file_id)
            .unwrap_or_else(|error| panic!("failed to load diagnostic file {file_id:?}: {error}"))
    };
    print_diagnostics_impl(&file_for_id, diagnostics, options);
}

/// Validate that a warning code is known.
fn validate_warning_code(code: &str) -> CliResult<String> {
    if DiagnosticRegistry::is_valid_warning_code(code) {
        Ok(code.to_string())
    } else {
        Err(CliError::message(format!("unknown warning code: {code}")))
    }
}

/// Validate that an error code is known.
fn validate_error_code(code: &str) -> CliResult<String> {
    if DiagnosticRegistry::is_valid_error_code(code) {
        Ok(code.to_string())
    } else {
        Err(CliError::message(format!("unknown error code: {code}")))
    }
}
