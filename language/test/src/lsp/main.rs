use std::process::ExitCode;

use clap::Parser;

use destack_test::harness::{Runner, TestOptions};
use destack_test::lsp::LspSuite;

/// Command line options for the applied LSP suite.
#[derive(Parser, Debug, Clone)]
#[command(name = "lsp", about = "Run Destack applied LSP tests")]
struct LspOptions {
    #[command(flatten)]
    test: TestOptions,
}

/// Run the applied LSP suite entrypoint.
fn main() -> ExitCode {
    let options = LspOptions::parse();
    let suite = LspSuite::load();
    Runner::run_suite(&suite, &options.test)
}
