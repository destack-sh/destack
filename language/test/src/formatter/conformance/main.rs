use std::process::ExitCode;

use clap::Parser;

use destack_test::formatter::conformance::{ConformanceHarnessSuite, ConformanceSelection};
use destack_test::harness::{Runner, TestOptions};

/// CLI options for the `formatter-conformance` test binary.
#[derive(Parser, Debug)]
#[command(
    name = "formatter-conformance",
    about = "Run formatter conformance tests"
)]
struct Args {
    /// Filter tests inside a selected conformance suite.
    #[arg(long)]
    suite_filter: Option<String>,

    /// Run Biome formatter suite.
    #[arg(long)]
    biome: bool,

    /// Run Prettier formatter suite.
    #[arg(long)]
    prettier: bool,

    /// Run oxfmt formatter suite.
    #[arg(long)]
    oxfmt: bool,

    /// Common test options.
    #[command(flatten)]
    test: TestOptions,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let mut test_options = args.test;
    test_options.continue_on_timeout = true;
    let selection = ConformanceSelection {
        biome: args.biome,
        prettier: args.prettier,
        oxfmt: args.oxfmt,
    };
    let suite = ConformanceHarnessSuite::new(
        selection,
        test_options.update_known_failures,
        args.suite_filter,
    );
    Runner::run_suite(&suite, &test_options)
}
