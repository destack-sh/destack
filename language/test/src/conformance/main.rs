use std::process::ExitCode;

use clap::Parser;

use destack_test::conformance::run_test262;
use destack_test::harness::TestOptions;

/// Conformance test specific options.
#[derive(Parser, Debug)]
#[command(name = "conformance", about = "Run parser conformance tests")]
struct Args {
    /// Update known-failures file with current failures.
    #[arg(long)]
    update_known_failures: bool,

    /// Run test262 suite (default if no suite specified).
    #[arg(long)]
    test262: bool,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let args = Args::parse();

    #[allow(clippy::overly_complex_bool_expr)] // (currently the only conformance suite)
    let should_run_test262 = args.test262 || true;

    let mut any_failed = false;

    if should_run_test262
        && run_test262(&args.test, args.update_known_failures) != ExitCode::SUCCESS
    {
        any_failed = true;
    }

    if any_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
