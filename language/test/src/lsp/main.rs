use std::process::ExitCode;

use clap::Parser;

use destack_test::core::{RunOptions, Runner};
use destack_test::lsp::LspSuite;

/// Command line options for the applied LSP suite.
#[derive(Parser, Debug, Clone)]
#[command(name = "lsp", about = "Run Destack applied LSP tests")]
struct LspOptions {
    #[command(flatten)]
    test: RunOptions,
}

/// Run the applied LSP suite entrypoint.
fn main() -> ExitCode {
    let options = LspOptions::parse();
    let suite = match LspSuite::load() {
        Ok(suite) => suite,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    Runner::run_suite(suite, &options.test)
}
